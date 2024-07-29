#![deny(unused_crate_dependencies)]

use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Query},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use cairo_vm::{
    program_hash::compute_program_hash_chain,
    types::{builtin_name::BuiltinName, layout_name::LayoutName, program::Program},
};
use dotenv::dotenv;
use layout_info::LAYOUT_INFO;
use serde::Deserialize;
use sqlx::Pool;
use sqlx::{postgres::PgPoolOptions, types::Uuid};
use starknet_crypto::FieldElement;
use std::{collections::HashSet, env, io::Cursor, sync::Arc};
use tokio_util::io::ReaderStream;
use tracing::info;

pub mod layout_info;

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
        .route(
            "/get-metadata",
            get({
                let db_pool = Arc::clone(&db_pool);
                move |program| get_metadata(program, db_pool)
            }),
        )
        .layer(DefaultBodyLimit::disable());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_program(
    program: Query<GetViaProgramHash>,
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
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}.json\"", program_hash),
        )
        .body(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(response)
}

async fn get_metadata(
    program: Query<GetViaProgramHash>,
    db_pool: Arc<Pool<sqlx::Postgres>>,
) -> Result<impl IntoResponse, StatusCode> {
    let program_hash = &program.program_hash;

    let row = sqlx::query!(
        "SELECT version, layout FROM programs WHERE hash = $1",
        program_hash
    )
    .fetch_one(&*db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let version = row.version;
    let layout = row.layout;

    // Create a JSON response with the version
    let json_response = serde_json::json!({ "version": version, "layout": layout});
    let body = Body::from(
        serde_json::to_string(&json_response).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    let response = Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(response)
}

#[derive(Deserialize)]
struct GetViaProgramHash {
    program_hash: String,
}

enum CairoCompilerVersion {
    Zero = 0,
    Two = 2,
}

impl From<i32> for CairoCompilerVersion {
    fn from(value: i32) -> Self {
        if value == 0 {
            Self::Zero
        } else if value == 2 {
            return Self::Two;
        } else {
            panic!("Unsupported Compiler Version")
        }
    }
}

async fn upload_program(
    mut multipart: Multipart,
    db_pool: Arc<Pool<sqlx::Postgres>>,
) -> Result<String, (StatusCode, String)> {
    let mut version: i32 = 0;
    let mut program_data = None;
    #[allow(unused_assignments)]
    let mut program_hash_hex = String::new();

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or_default().to_string();
        if name == "program" {
            let raw_data = field.bytes().await.unwrap();
            version = get_compiler_version(raw_data.to_vec()).unwrap();
            info!("Compiler version: {}", version);
            program_data = Some(raw_data);
        }
    }

    if let Some(data) = program_data {
        info!("Uploading program with version {}", version);
        let version = CairoCompilerVersion::from(version);
        let (id, code, version, builtins, layout) = match version {
            CairoCompilerVersion::Two => {
                let casm: CasmContractClass = serde_json::from_slice(&data).unwrap();
                let program_hash = casm.compiled_class_hash();
                let convert = FieldElement::from_bytes_be(&program_hash.to_be_bytes()).unwrap();
                program_hash_hex = format!("{:#x}", convert);
                info!("Program hash: {}", program_hash_hex);

                let mut builtin_set = HashSet::new();

                casm.entry_points_by_type
                    .external
                    .iter()
                    .for_each(|x| builtin_set.extend(x.clone().builtins));

                let builtin_vec: Vec<String> = builtin_set.iter().map(|x| x.to_string()).collect();
                info!("Builtins: {:?}", builtin_vec);
                let layout = get_best_cairo_layout(&builtin_vec);
                info!("Layout: {:?}", layout);

                let id = Uuid::from_bytes(uuid::Uuid::new_v4().to_bytes_le());
                (
                    id,
                    data.as_ref(),
                    version as i32,
                    builtin_vec,
                    layout.to_str(),
                )
            }
            CairoCompilerVersion::Zero => {
                let program =
                    Program::from_bytes(&data, Some("main")).expect("Could not load program.");
                let stripped_program = program.get_stripped_program().unwrap();
                let builtins: Vec<String> = stripped_program
                    .builtins
                    .iter()
                    .map(|x| x.to_str().to_string())
                    .collect();
                info!("Builtins: {:?}", builtins);
                let layout = get_best_cairo_layout(&builtins);
                info!("Layout: {:?}", layout);
                let program_hash = compute_program_hash_chain(&stripped_program, 0)
                    .expect("Failed to compute program hash.");

                program_hash_hex = format!("{:#x}", program_hash);
                info!("Program Hash: {}", program_hash_hex);

                let id = Uuid::from_bytes(uuid::Uuid::new_v4().to_bytes_le());
                (id, data.as_ref(), version as i32, builtins, layout.to_str())
            }
        };

        let result = sqlx::query!(
            "INSERT INTO programs (id, hash, code, version, builtins, layout) VALUES ($1, $2, $3, $4, $5, $6)",
            id, program_hash_hex, code, version, &builtins, layout
        )
        .execute(&*db_pool)
        .await;

        if let Err(err) = result {
            match err.as_database_error() {
                Some(pg_err) if pg_err.code() == Some(std::borrow::Cow::Borrowed("23505")) => {
                    return Err((
                        StatusCode::CONFLICT,
                        "Duplicate data error: The entry already exists.".to_string(),
                    ));
                }
                _ => {
                    return Err((StatusCode::BAD_REQUEST, err.to_string()));
                }
            };
        }
    }

    Ok(program_hash_hex)
}

fn get_compiler_version(bytes: Vec<u8>) -> Result<i32, Box<dyn std::error::Error>> {
    let json_str = String::from_utf8(bytes)?;

    // Parse the JSON string to a serde_json::Value
    let json_value: serde_json::Value = serde_json::from_str(&json_str)?;

    // Access the "compiler_version" field and extract its value
    if let Some(version) = json_value.get("compiler_version").and_then(|v| v.as_str()) {
        let full_compiler_version = version.to_string();
        let version = full_compiler_version.split('.').collect::<Vec<&str>>()[0]
            .parse::<i32>()
            .unwrap();
        Ok(version)
    } else {
        Err("compiler_version field not found or not a uint".into())
    }
}

fn get_best_cairo_layout(builtins: &[String]) -> LayoutName {
    let required_builtins: HashSet<BuiltinName> = builtins
        .iter()
        .filter_map(|b| BuiltinName::from_str(b))
        .collect();

    let mut best_layout = None;
    let mut min_trace_columns = u32::MAX;

    for (layout_name, (n_trace_columns, layout_builtins)) in LAYOUT_INFO.iter() {
        if layout_builtins.is_superset(&required_builtins) && *n_trace_columns < min_trace_columns {
            min_trace_columns = *n_trace_columns;
            best_layout = Some(*layout_name);
        }
    }

    best_layout.unwrap_or(LayoutName::starknet_with_keccak)
}
