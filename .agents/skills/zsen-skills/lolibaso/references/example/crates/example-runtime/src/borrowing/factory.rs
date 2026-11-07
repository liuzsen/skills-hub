use anyhow::Context;
use borrowing_application::{borrow_device, register};
use borrowing_http::devices;
use borrowing_sqlite::SqliteRepository;
use lolibaso::inject::TypeBuilder;

use super::{device_ids::DeviceIds, sqlite_database::SqliteDatabase};

/// Shares database configuration; each owned case receives a fresh connection.
#[derive(Clone)]
pub struct UseCaseFactory {
    pub database: SqliteDatabase,
}

// 省略查询、归还、注销、内部维修 HTTP 与进程内维修协作的 Factory 实现。
impl devices::register::Factory for UseCaseFactory {
    type Case = register::Register<SqliteRepository, DeviceIds>;

    async fn create(&self) -> anyhow::Result<Self::Case> {
        TypeBuilder::new()
            .add_instance(self.database.connect().await?)
            .build_async::<Self::Case, _>()
            .await
            .context("creating register use case")
    }
}

impl devices::borrow::Factory for UseCaseFactory {
    type Case = borrow_device::BorrowDevice<SqliteRepository>;

    async fn create(&self) -> anyhow::Result<Self::Case> {
        TypeBuilder::new()
            .add_instance(self.database.connect().await?)
            .build_async::<Self::Case, _>()
            .await
            .context("creating borrow device use case")
    }
}
