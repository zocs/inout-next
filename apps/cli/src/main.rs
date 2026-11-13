use clap::Parser;
use share_domain::Product;
use share_http::{start, ServerConfig};
use std::{net::SocketAddr, path::PathBuf, process::ExitCode};

#[derive(Parser)]
#[command(version, about = "Independent local file-sharing server")]
struct Arguments {
    #[arg(long)]
    root: PathBuf,
    #[arg(long, default_value = "127.0.0.1:5000")]
    listen: SocketAddr,
    #[arg(long)]
    web_dir: Option<PathBuf>,
    #[arg(long)]
    key_file: Option<PathBuf>,
    #[arg(long)]
    writable: bool,
    #[arg(long, value_delimiter = ',', default_value = "files,uploads,manage")]
    modules: Vec<String>,
    #[arg(long, default_value_t = 2147483648_u64)]
    max_upload_bytes: u64,
    #[arg(long)]
    product_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> ExitCode {
    match run(Arguments::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
async fn run(arguments: Arguments) -> Result<(), String> {
    let mut config = ServerConfig::new(arguments.root, arguments.listen);
    config.web_dir = arguments.web_dir;
    config.writable = arguments.writable;
    config.modules = arguments.modules;
    config.max_upload_bytes = arguments.max_upload_bytes;
    if let Some(path) = arguments.product_file {
        config.product = Product::load(path)
            .map_err(|error| format!("Could not load product manifest: {error}"))?;
    }
    if let Some(path) = arguments.key_file {
        let key = tokio::fs::read_to_string(path)
            .await
            .map_err(|_| "Could not read access key file".to_owned())?;
        config.key = Some(key.trim_end_matches(['\r', '\n']).to_owned());
    }
    let server = start(config).await.map_err(|error| error.to_string())?;
    println!("Listening on http://{}", server.address());
    let signal = tokio::signal::ctrl_c().await;
    server.stop().await.map_err(|error| error.to_string())?;
    signal.map_err(|_| "Could not listen for Ctrl-C".to_owned())
}
