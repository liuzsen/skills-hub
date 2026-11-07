use std::{
    env,
    net::{Ipv4Addr, SocketAddr},
};

use anyhow::Context;

pub struct Config {
    pub database_path: String,
    pub port: u16,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let database_path = env_or_default("BORROWING_DATABASE_PATH", ".local/borrowing.sqlite")?;
        let port = env_or_default("BORROWING_PORT", "3000")?
            .parse::<u16>()
            .context("BORROWING_PORT must be an unsigned 16-bit port number")?;
        Ok(Self {
            database_path,
            port,
        })
    }

    pub fn address(&self) -> SocketAddr {
        (Ipv4Addr::LOCALHOST, self.port).into()
    }
}

fn env_or_default(name: &str, default: &str) -> anyhow::Result<String> {
    match env::var(name) {
        Ok(value) => Ok(value),
        Err(env::VarError::NotPresent) => Ok(default.to_owned()),
        Err(error) => Err(error).with_context(|| format!("reading {name}")),
    }
}
