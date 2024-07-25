use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Query},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use dotenv::dotenv;
use serde::Deserialize;
use sqlx::Pool;
use sqlx::{postgres::PgPoolOptions, types::Uuid};
use std::{env, io::Cursor, sync::Arc};
use tokio_util::io::ReaderStream;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // Load environment variables from .env file
    dotenv().ok();

    // Read database connection info from environment variables
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Connect to the database
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool.");
    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let db_pool = Arc::new(pool);

    // build our application with a route
    let app = Router::new()
        .route(
            "/get-program",
            get({
                let db_pool = Arc::clone(&db_pool);
                move |program| get_program(program, db_pool)
            }),
        )
        .route(
            "/upload-program",
            post({
                let db_pool = Arc::clone(&db_pool);
                move |multipart| upload_program(multipart, db_pool)
            }),
        )
        .layer(DefaultBodyLimit::disable());

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_program(
    program: Query<GetProgram>,
    db_pool: Arc<Pool<sqlx::Postgres>>,
) -> Result<impl IntoResponse, StatusCode> {
    let program_hash = &program.program_hash;

    let row = sqlx::query!("SELECT code FROM programs WHERE hash = $1", program_hash)
        .fetch_one(&*db_pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let code = row.code;

    let stream = ReaderStream::new(Cursor::new(code));
    let body = Body::from_stream(stream);

    let response = Response::builder()
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", program_hash),
        )
        .body(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(response)
}

#[derive(Deserialize)]
struct GetProgram {
    program_hash: String,
}

async fn upload_program(
    mut multipart: Multipart,
    db_pool: Arc<Pool<sqlx::Postgres>>,
) -> Result<&'static str, StatusCode> {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name();
        println!("Field name: {:?}", name);
        let data = field.text().await.unwrap();
        // println!("Field data: {:?}", data);
        let casm: CasmContractClass = serde_json::from_str(&data).unwrap();
        let program_hash = casm.compiled_class_hash().to_string();

        // Generate a UUID for the id field
        let id = Uuid::from_bytes(uuid::Uuid::new_v4().to_bytes_le());

        let result = sqlx::query!(
            "INSERT INTO programs (id, hash, code) VALUES ($1, $2, $3)",
            id,
            program_hash,
            data.as_bytes()
        )
        .execute(&*db_pool)
        .await;

        if result.is_err() {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    Ok("success")
}
