use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, ensure};
use borrowing_sqlite::Connection;
use diesel_async::{AsyncConnection, SimpleAsyncConnection};

/// Shares a file path, never a connection. Migrations belong to each module.
#[derive(Clone)]
pub(crate) struct SqliteFile {
    path: Arc<str>,
}

impl SqliteFile {
    pub(crate) async fn prepare(path: &str) -> anyhow::Result<Self> {
        let path = resolve_path(path).await?;
        Ok(Self {
            path: Arc::from(path.to_str().context("database path must be UTF-8")?),
        })
    }

    pub(crate) async fn connect(&self) -> anyhow::Result<Connection> {
        let mut connection = Connection::establish(&self.path)
            .await
            .context("opening SQLite connection")?;
        connection
            .batch_execute("PRAGMA busy_timeout = 5000;")
            .await
            .context("setting SQLite busy timeout")?;
        Ok(connection)
    }
}

// 省略 borrowing/maintenance 双数据库隔离检查；当前组合根只打开借用数据库。

async fn resolve_path(database_path: &str) -> anyhow::Result<PathBuf> {
    ensure!(
        !database_path.trim().is_empty()
            && database_path != ":memory:"
            && !database_path.starts_with("file:")
            && !database_path.starts_with("sqlite:")
            && !database_path.contains('\0'),
        "database must be a nonempty SQLite file path, not an in-memory database or URI"
    );
    let absolute =
        std::path::absolute(Path::new(database_path)).context("resolving database path")?;
    let parent = absolute
        .parent()
        .context("database path must have a parent directory")?;
    tokio::fs::create_dir_all(parent)
        .await
        .context("creating database directory")?;
    let parent = tokio::fs::canonicalize(parent)
        .await
        .context("resolving database directory")?;
    let path = parent.join(
        absolute
            .file_name()
            .context("database path must name a file")?,
    );
    match tokio::fs::symlink_metadata(&path).await {
        Ok(_) => tokio::fs::canonicalize(&path)
            .await
            .context("resolving existing database file"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path),
        Err(error) => Err(error).context("inspecting database path"),
    }
}
