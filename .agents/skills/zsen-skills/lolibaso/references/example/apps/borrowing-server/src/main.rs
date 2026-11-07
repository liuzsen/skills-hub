mod config;

use std::process::ExitCode;

use anyhow::Context;
use borrowing_http::router;
use config::Config;
use example_runtime::{
    borrowing::{SqliteDatabase, UseCaseFactory},
    request_tracing,
};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    match run().await {
        Ok(()) => {
            tracing::info!("server stopped");
            ExitCode::SUCCESS
        }
        Err(error) => {
            tracing::error!(error = %format_args!("{error:#}"), "server failed");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> anyhow::Result<()> {
    tracing::info!("starting borrowing server");
    tracing::warn!("Trusted local development only: X-Dev-User is NOT authentication");
    let config = Config::from_env()?;
    let database = SqliteDatabase::initialize(&config.database_path).await?;
    let factory = UseCaseFactory { database };
    // 仅装配借用与登记；省略内部维修路由的合并。
    let app = router(factory);
    let listener = TcpListener::bind(config.address())
        .await
        .context("binding HTTP listener")?;
    tracing::info!(address = %listener.local_addr()?, "listening for HTTP requests");
    axum::serve(listener, request_tracing::with_request_tracing(app))
        .with_graceful_shutdown(shutdown())
        .await
        .context("serving HTTP")
}

async fn shutdown() {
    match tokio::signal::ctrl_c().await {
        Ok(()) => tracing::info!("Ctrl-C received; shutting down"),
        Err(error) => tracing::error!(%error, "installing Ctrl-C handler failed; shutting down"),
    }
}
