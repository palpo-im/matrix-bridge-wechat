pub mod config;
pub mod database;
pub mod wechat;
pub mod bridge;
pub mod util;
pub mod formatter;
pub mod matrix;
pub mod web;
pub mod crypto;
pub mod error;
pub mod metrics;

pub const NAME: &str = "matrix-wechat";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
