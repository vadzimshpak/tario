pub mod client;
pub mod server;

use std::path::PathBuf;
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct UploadResponse {
    pub message: String,
    pub path: String
}

#[derive(Parser, Debug)]
#[command(name = "tario")]
#[command(version)]
#[command(about = "Upload files and directories everywhere")]
pub struct Cli {
    pub path: Option<PathBuf>,

    #[arg(short, long)]
    pub server: bool,

    #[arg(long, short, default_value = "0.0.0.0")]
    pub url: String,

    #[arg(long, short, default_value = "3000")]
    pub port: u16,

    #[arg(long, short, default_value = "186.246.27.3:3000")]
    pub bucket_url: String,
}
