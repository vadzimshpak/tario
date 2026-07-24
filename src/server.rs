use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use axum::{
    body::Body,
    extract::{Path as AxumPath, State},
    http::{
        StatusCode,
        header::{CONTENT_LENGTH, CONTENT_TYPE, CONTENT_DISPOSITION}
    },
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};

use futures_util::TryStreamExt;
use tokio::io::{self, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_util::io::{ReaderStream, StreamReader};
use sha2::{Digest, Sha256};

use crate::UploadResponse;
use crate::Cli;

#[derive(Clone)]
struct AppState {
    upload_directory: Arc<PathBuf>,
}

pub async fn run_server(args: Cli) -> io::Result<()> {
    let upload_directory = PathBuf::from("./uploads");
    tokio::fs::create_dir_all(&upload_directory).await?;

    let state = AppState {
        upload_directory: Arc::new(upload_directory),
    };

    let app = Router::new()
        .route("/{hash}", get(download))
        .route("/upload", post(upload))
        .with_state(state);


    let address = &format!("{}:{}", args.url, args.port);
    let listener = TcpListener::bind(address).await?;

    println!("Tario server started at http://{address}");
    println!("Uploads directory: ./uploads");

    axum::serve(listener, app).await
}

async fn download(
    State(state): State<AppState>,
    AxumPath(hash): AxumPath<String>
) -> Result<Response, (StatusCode, String)> {
    if !is_valid_hash(&hash) {
        return Err((StatusCode::BAD_REQUEST, "Invalid hash".to_string()));
    }

    let file_path = state
        .upload_directory
        .join(format!("{hash}.tar.gz"));

    println!("Downloading file: {file_path:?}");

    let file = match tokio::fs::File::open(&file_path).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err((StatusCode::NOT_FOUND, "File not found".to_string()));
        }
        Err(error) => return Err((StatusCode::INTERNAL_SERVER_ERROR, error.to_string())),
    };

    let metadata = file
        .metadata()
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/gzip")
        .header(CONTENT_LENGTH, metadata.len())
        .header(CONTENT_DISPOSITION, "attachment; filename=archive.tar.gz")
        .body(body)
        .map_err(internal_error)
}

fn is_valid_hash(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}

async fn upload(
    State(state): State<AppState>,
    body: Body,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let upload_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(internal_error)?
        .as_nanos();

    let upload_name = hash_name(upload_id.to_string());

    let final_path = state
        .upload_directory
        .join(format!("{upload_name}.tar.gz"));

    // Временное имя, пока загрузка не завершена.
    let temporary_path = state
        .upload_directory
        .join(format!("{upload_name}.tar.gz.part"));

    let stream = body
        .into_data_stream()
        .map_err(io::Error::other);

    let mut reader = StreamReader::new(stream);

    let mut file = tokio::fs::File::create(&temporary_path)
        .await
        .map_err(internal_error)?;

    // Потоково копируем HTTP body в файл.
    let bytes_written = match tokio::io::copy(&mut reader, &mut file).await {
        Ok(size) => size,

        Err(error) => {
            drop(file);
            let _ = tokio::fs::remove_file(&temporary_path).await;

            return Err((
                StatusCode::BAD_REQUEST,
                format!("Failed to receive archive: {error}"),
            ));
        }
    };

    // Гарантируем передачу буферизированных данных ОС.
    file.flush()
        .await
        .map_err(internal_error)?;

    drop(file);

    // После успешной загрузки переименовываем временный файл.
    tokio::fs::rename(&temporary_path, &final_path)
        .await
        .map_err(internal_error)?;

    let response = UploadResponse {
        message: format!(
            "Archive uploaded successfully: {bytes_written} bytes"
        ),
        path: upload_name,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

fn internal_error(error: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", error))
}

fn hash_name(name: String) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());

    hex::encode(hasher.finalize())
}