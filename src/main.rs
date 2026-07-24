use std::error::Error;

use clap::Parser;

use tario::Cli;
use tario::client::run_client;
use tario::server::run_server;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    match args.server {
        true => run_server(args).await?,
        false => run_client(args).await?,
    }

    Ok(())
}
