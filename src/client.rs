use std::error::Error;
use std::path::{Path};

use flate2::write::GzEncoder;
use reqwest::Body;
use tar::Builder;
use tokio::{io, task};
use tokio_util::io::{ReaderStream, SyncIoBridge};

use crate::UploadResponse;
use crate::Cli;


async fn upload_directory(source: Box<Path>, args: Cli) -> Result<(), Box<dyn Error>> {
    let (writer, reader) = io::duplex(64 * 1024);

    let archive_task = task::spawn_blocking(move || -> io::Result<()> {
        let writer = SyncIoBridge::new(writer);

        // Directory -> Gzip encoder
        let gzip = GzEncoder::new(writer, flate2::Compression::default());

        // Gzip encoder -> Tar builder
        let mut tar = Builder::new(gzip);

        tar.append_dir_all(".", source.as_ref())?;
        tar.finish()?;

        let gzip = tar.into_inner()?;
        let _writer = gzip.finish();

        Ok(())
    });

    let upload_url = format!("http://{}/upload", args.bucket_url);

    let stream = ReaderStream::new(reader);
    let body = Body::wrap_stream(stream);
    let client = reqwest::Client::new();
    let request_result = client
        .post(upload_url)
        .header("Content-Type", "application/gzip")
        .header("Content-Disposition", "attachment; filename=\"upload.tar.gz\"")
        .body(body)
        .send()
        .await;

    archive_task
        .await
        .map_err(|e| io::Error::other(format!("Archive task failed: {e}")))??;

    let response = request_result?.error_for_status()?;
    let status = response.status();
    let data = response.json::<UploadResponse>().await?;

    if status.is_success() {
        println!("Uploaded: http://{}/{}", args.bucket_url, data.path);
        Ok(())
    } else {
        Err(io::Error::other(format!("Something went wrong")))?
    }
}

pub async fn run_client(args: Cli) -> Result<(), Box<dyn Error>> {
    if let Some(path) = args.path.clone() {
        if !path.exists() {
            eprintln!("Path {} is not exists", path.display());
        }

        println!("Source: {}", path.display());
        upload_directory(path.into_boxed_path(), args).await?;
    } else {
        println!("No source specified.");
    }

    Ok(())
}
