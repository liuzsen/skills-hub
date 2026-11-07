mod device_row;
// 省略维修仓储模块声明；维修状态的行映射仍用于识别借用拒绝。
mod repository;
mod schema;

use anyhow::{Context, ensure};
use diesel_async::{SimpleAsyncConnection, TransactionManager};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

pub use repository::DeviceSnapshot;

/// The composition root supplies an exclusively owned connection and Tokio runtime.
pub type Connection =
    diesel_async::sync_connection_wrapper::SyncConnectionWrapper<diesel::SqliteConnection>;

type SqliteTransactionManager =
    diesel_async::sync_connection_wrapper::SyncTransactionManagerWrapper<
        diesel::connection::AnsiTransactionManager,
    >;

/// An exclusive connection context, not a shared handle or connection pool.
///
/// The caller supplies an idle connection. Use with a Tokio multi-thread runtime.
/// Cancelled transactions and failed transaction control discard the connection;
/// further use of that repository returns a technical error. Cancellation in the
/// transaction body rolls back on close; during COMMIT the outcome may already
/// be committed, so cancellation does not imply that retrying is safe.
pub struct SqliteRepository {
    connection: Option<Connection>,
    in_transaction: bool,
}

#[lolibaso::inject::inject]
impl SqliteRepository {
    pub async fn new(mut connection: Connection) -> anyhow::Result<Self> {
        ensure!(
            !SqliteTransactionManager::is_broken_transaction_manager(&mut connection),
            "SQLite repository requires an idle, healthy connection"
        );
        connection
            .batch_execute("PRAGMA foreign_keys = ON;")
            .await
            .context("enabling SQLite foreign keys")?;
        Ok(Self {
            connection: Some(connection),
            in_transaction: false,
        })
    }
}

// 教学摘录不附 migrations/：上游迁移负责唯一设备编号、外键及状态约束。
// 保留原嵌入路径和初始化签名，供阅读资源装配；此摘录不能独立编译。
const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Apply the versioned schema on a blocking worker before serving use cases.
pub async fn migrate(connection: &mut Connection) -> anyhow::Result<()> {
    connection
        .spawn_blocking(|connection| {
            Ok(connection
                .run_pending_migrations(MIGRATIONS)
                .map(|versions| versions.len())
                .map_err(anyhow::Error::from_boxed))
        })
        .await??;
    Ok(())
}
