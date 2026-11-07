use anyhow::{Context, ensure};
use borrowing_application::{CirculationRepository, register::InsertConflict};
use borrowing_domain::{
    device::{Device, DeviceId},
    movement::{Movement, MovementKind},
};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, SelectableHelper};
use diesel_async::{RunQueryDsl, TransactionManager};
use lolibaso::{
    biz::BizResult,
    repository::{
        EntitySnapshot, InsertOutcome, Repository, ScopedBoxFuture, TransactionalRepository,
        UpdateOutcome,
    },
};

use crate::{
    Connection, SqliteRepository, SqliteTransactionManager,
    device_row::{DeviceRow, DeviceWriteRow},
    schema::{devices, movements},
};

pub struct DeviceSnapshot {
    pub(crate) device: Device,
    pub(crate) version: i64,
}

impl EntitySnapshot<Device> for DeviceSnapshot {
    fn entity(&self) -> &Device {
        &self.device
    }

    fn entity_mut(&mut self) -> &mut Device {
        &mut self.device
    }

    fn into_entity(self) -> Device {
        self.device
    }
}

impl SqliteRepository {
    pub(crate) fn connection(&mut self) -> anyhow::Result<&mut Connection> {
        self.connection
            .as_mut()
            .context("SQLite connection was discarded after an interrupted or failed transaction")
    }
}

impl Repository<Device> for SqliteRepository {
    type Snapshot = DeviceSnapshot;
    type InsertConflict = InsertConflict;

    async fn find(&mut self, id: &DeviceId) -> anyhow::Result<Option<DeviceSnapshot>> {
        devices::table
            .find(id.0.to_string())
            .select(DeviceRow::as_select())
            .first::<DeviceRow>(self.connection()?)
            .await
            .optional()
            .context("loading device")?
            .map(DeviceRow::into_snapshot)
            .transpose()
    }

    async fn insert(&mut self, device: &Device) -> anyhow::Result<InsertOutcome<InsertConflict>> {
        let row = DeviceWriteRow::from_device(device, 0);
        let inserted = diesel::insert_into(devices::table)
            .values(&row)
            .on_conflict(devices::number)
            .do_nothing()
            .execute(self.connection()?)
            .await
            .context("inserting device")?;
        if inserted == 1 {
            return Ok(InsertOutcome::Saved);
        }
        // A simultaneous number and ID conflict must not hide a broken ID generator.
        let id_exists = devices::table
            .find(&row.id)
            .select(devices::id)
            .first::<String>(self.connection()?)
            .await
            .optional()
            .context("checking device ID collision")?
            .is_some();
        ensure!(!id_exists, "generated device ID collision");
        Ok(InsertOutcome::Conflict(InsertConflict::NumberTaken))
    }

    async fn update(
        &mut self,
        snapshot: DeviceSnapshot,
    ) -> anyhow::Result<UpdateOutcome<DeviceSnapshot>> {
        let next_version = snapshot
            .version
            .checked_add(1)
            .context("device version exhausted")?;
        let row = DeviceWriteRow::from_device(&snapshot.device, next_version);
        let updated = diesel::update(
            devices::table
                .find(&row.id)
                .filter(devices::version.eq(snapshot.version)),
        )
        .set((
            devices::number.eq(row.number),
            devices::status.eq(row.status),
            devices::borrower_id.eq(&row.borrower_id),
            devices::version.eq(row.version),
            devices::maintenance_work_order_id.eq(&row.maintenance_work_order_id),
        ))
        .execute(self.connection()?)
        .await
        .context("updating device snapshot")?;
        if updated == 0 {
            return Ok(UpdateOutcome::StaleState);
        }
        Ok(UpdateOutcome::Updated(DeviceSnapshot {
            device: snapshot.device,
            version: next_version,
        }))
    }
}

// 省略查询用例 Reader 实现；find 与 Snapshot 已展示实体读取和重建。

impl TransactionalRepository<Device> for SqliteRepository {
    async fn transaction<'a, Tx, T, Err>(&'a mut self, tx: Tx) -> BizResult<T, Err>
    where
        for<'repo> Tx:
            Send + FnOnce(&'repo mut Self) -> ScopedBoxFuture<'a, 'repo, BizResult<T, Err>>,
        T: Send,
        Err: Send,
    {
        ensure!(
            !self.in_transaction,
            "nested transactions are not supported by this repository"
        );
        // Move ownership before the first await: cancellation/panic drops this context
        // instead of leaving an open transaction available to the original repository.
        let mut transaction = Self {
            connection: Some(
                self.connection
                    .take()
                    .context("SQLite connection unavailable")?,
            ),
            in_transaction: true,
        };
        transaction
            .connection()?
            .spawn_blocking(|connection| {
                diesel::connection::AnsiTransactionManager::begin_transaction_sql(
                    connection,
                    "BEGIN IMMEDIATE",
                )
            })
            .await
            .context("beginning SQLite circulation transaction")?;

        let result = tx(&mut transaction).await;
        if matches!(&result, Ok(Ok(_))) {
            SqliteTransactionManager::commit_transaction(transaction.connection()?)
                .await
                .context("committing SQLite circulation transaction")?;
        } else {
            SqliteTransactionManager::rollback_transaction(transaction.connection()?)
                .await
                .with_context(|| match &result {
                    Err(error) => {
                        format!("rolling back SQLite circulation transaction after: {error:#}")
                    }
                    _ => "rolling back SQLite circulation transaction after business refusal"
                        .to_owned(),
                })?;
        }
        self.connection = transaction.connection.take();
        result
    }
}

impl CirculationRepository for SqliteRepository {
    async fn append_movement(&mut self, movement: Movement) -> anyhow::Result<()> {
        ensure!(
            self.in_transaction,
            "movement requires an active circulation transaction"
        );
        let kind = match movement.kind {
            MovementKind::Borrow => "borrow",
            MovementKind::Return => "return",
        };
        diesel::insert_into(movements::table)
            .values((
                movements::device_id.eq(movement.device_id.0.to_string()),
                movements::actor_id.eq(movement.actor_id.0.to_string()),
                movements::kind.eq(kind),
            ))
            .execute(self.connection()?)
            .await
            .context("appending device movement")?;
        Ok(())
    }
}
