use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Query},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use cairo_vm::{program_hash::compute_program_hash_chain, types::program::Program};
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
    let mut version = 0;
    let mut program_data = None;

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or_default().to_string();
        if name == "version" {
            version = field.text().await.unwrap().parse::<i32>().unwrap_or(0);
        } else if name == "program" {
            program_data = Some(field.bytes().await.unwrap());
        }
    }

    if let Some(data) = program_data {
        println!("Uploading program with version {}", version);
        if version == 1 {
            let casm: CasmContractClass = serde_json::from_slice(&data).unwrap();
            let program_hash = casm.compiled_class_hash().to_string();

            println!("Program hash: {}", program_hash);
            // Generate a UUID for the id field
            let id = Uuid::from_bytes(uuid::Uuid::new_v4().to_bytes_le());

            let result = sqlx::query!(
                "INSERT INTO programs (id, hash, code, version) VALUES ($1, $2, $3, $4)",
                id,
                program_hash,
                data.as_ref(),
                version
            )
            .execute(&*db_pool)
            .await;

            if result.is_err() {
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }

        if version == 0 {
            let program =
            Program::from_bytes(&data, Some("main"))
                .expect("Could not load program. Did you compile the sample programs? Run `make test` in the root directory.");
            let stripped_program = program.get_stripped_program().unwrap();
            let bootloader_version = 0;
            let program_hash = compute_program_hash_chain(&stripped_program, bootloader_version)
                .expect("Failed to compute program hash.");

            let program_hash_hex = format!("{:#x}", program_hash);
            println!("{}", program_hash_hex);
            // Generate a UUID for the id field
            let id = Uuid::from_bytes(uuid::Uuid::new_v4().to_bytes_le());

            let result = sqlx::query!(
                "INSERT INTO programs (id, hash, code, version) VALUES ($1, $2, $3, $4)",
                id,
                program_hash_hex,
                data.as_ref(),
                version
            )
            .execute(&*db_pool)
            .await;

            if result.is_err() {
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
        // Add other version handling if needed
    } else {
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok("success")
}
