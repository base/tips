use alloy_rpc_types_engine::JwtSecret;
use clap::Parser;
use eyre::Result;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "shadow-boost")]
#[command(about = "Shadow builder proxy for driving builder from non-sequencer op-node")]
pub struct Config {
    #[arg(long, env = "BUILDER_URL")]
    pub builder_url: String,

    #[arg(long, env = "BUILDER_JWT_SECRET")]
    pub builder_jwt_secret: PathBuf,

    #[arg(long, env = "LISTEN_ADDR", default_value = "127.0.0.1:8554")]
    pub listen_addr: String,

    #[arg(long, env = "TIMEOUT_MS", default_value = "2000")]
    pub timeout_ms: u64,
}

impl Config {
    pub fn load_jwt_secret(&self) -> Result<JwtSecret> {
        Ok(JwtSecret::from_file(&self.builder_jwt_secret)?)
    }
}
