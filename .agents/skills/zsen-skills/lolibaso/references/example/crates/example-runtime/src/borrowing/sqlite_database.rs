use anyhow::Context;
use borrowing_sqlite::{Connection, migrate};

use crate::sqlite_file::SqliteFile;

/// Shares only an immutable absolute path, never a connection or repository.
#[derive(Clone)]
pub struct SqliteDatabase {
    file: SqliteFile,
}

impl SqliteDatabase {
    pub async fn initialize(path: &str) -> anyhow::Result<Self> {
        let database = Self {
            file: SqliteFile::prepare(path).await?,
        };
        let mut connection = database.connect().await?;
        migrate(&mut connection)
            .await
            .context("migrating borrowing SQLite database")?;
        Ok(database)
    }

    pub async fn connect(&self) -> anyhow::Result<Connection> {
        self.file.connect().await
    }
}
