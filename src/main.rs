use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, error, Level};
use tracing_subscriber::FmtSubscriber;

mod config;
mod database;
mod wechat;
mod bridge;
mod formatter;
mod util;
mod matrix;
mod web;
mod crypto;
mod error;
mod metrics;

use config::Config;
use bridge::WechatBridge;

#[derive(Parser, Debug)]
#[command(name = "matrix-wechat")]
#[command(version)]
#[command(about = "A Matrix-WeChat puppeting bridge")]
struct Args {
    /// Path to config file
    #[arg(short, long, default_value = "config.yaml")]
    config: PathBuf,

    /// Generate example config and exit
    #[arg(long)]
    generate_config: bool,
}

const EXAMPLE_CONFIG: &str = include_str!("../example-config.yaml");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    if args.generate_config {
        println!("{}", EXAMPLE_CONFIG);
        return Ok(());
    }

    FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .pretty()
        .init();
    
    info!("Starting Matrix-WeChat bridge v{}", env!("CARGO_PKG_VERSION"));
    
    let config_path = args.config.to_string_lossy();
    info!("Loading config from {}", config_path);
    
    let config = match Config::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to load config: {}", e);
            return Err(e);
        }
    };

    let bridge = WechatBridge::new(config.clone()).await?;
    let bridge = Arc::new(bridge);
    
    info!("Bridge initialized, starting services...");

    let web_router = web::create_appservice_router(bridge.clone());
    let web_addr: &'static str = Box::leak(format!("{}:{}", config.appservice.hostname, config.appservice.port).into_boxed_str());
    info!("Web server will listen on {}", web_addr);
    
    let bridge_for_task = bridge.clone();
    let web_handle = tokio::spawn(async move {
        use salvo::conn::TcpListener;
        use salvo::prelude::*;
        
        let listener = TcpListener::new(web_addr).bind().await;
        Server::new(listener).serve(web_router).await;
    });

    let bridge_handle = tokio::spawn(async move {
        if let Err(e) = bridge_for_task.start().await{
            error!("Bridge error: {}", e);
        }
    });

    tokio::select! {
        _ = bridge_handle => {
            info!("Bridge task ended");
        }
        _ = web_handle => {
            info!("Web server task ended");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }
    
    bridge.stop().await;
    info!("Bridge stopped");
    
    Ok(())
}
