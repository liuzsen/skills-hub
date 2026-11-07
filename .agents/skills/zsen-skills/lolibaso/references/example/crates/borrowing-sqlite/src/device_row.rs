use anyhow::{Context, anyhow, ensure};
use borrowing_domain::device::{
    ActorId, Device, DeviceId, DeviceNumber, DeviceState, DeviceStatus, WorkOrderId,
};
use diesel::{Insertable, Queryable, Selectable};
use lolibaso::entity::{Entity, Reconstitute};
use uuid::Uuid;

use crate::{repository::DeviceSnapshot, schema::devices};

#[derive(Queryable, Selectable)]
#[diesel(table_name = devices)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct DeviceRow {
    pub id: String,
    pub number: String,
    pub status: String,
    pub borrower_id: Option<String>,
    pub version: i64,
    pub maintenance_work_order_id: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = devices)]
pub(crate) struct DeviceWriteRow<'a> {
    pub id: String,
    pub number: &'a str,
    pub status: &'static str,
    pub borrower_id: Option<String>,
    pub version: i64,
    pub maintenance_work_order_id: Option<String>,
}

impl<'a> DeviceWriteRow<'a> {
    pub fn from_device(device: &'a Device, version: i64) -> Self {
        let view = device.readonly();
        let (status, borrower_id, maintenance_work_order_id) = match *view.status {
            DeviceStatus::Available => ("available", None, None),
            DeviceStatus::Borrowed { borrower } => ("borrowed", Some(borrower.0.to_string()), None),
            DeviceStatus::UnderMaintenance { work_order_id } => {
                ("available", None, Some(work_order_id.0.to_string()))
            }
            DeviceStatus::Deregistered => ("deregistered", None, None),
        };
        Self {
            id: view.id.0.to_string(),
            number: view.number.as_str(),
            status,
            borrower_id,
            version,
            maintenance_work_order_id,
        }
    }
}

impl DeviceRow {
    pub fn into_snapshot(self) -> anyhow::Result<DeviceSnapshot> {
        ensure!(self.version >= 0, "invalid persisted device version");
        let id = DeviceId(Uuid::parse_str(&self.id).context("invalid persisted device ID")?);
        let number = DeviceNumber::parse(self.number)
            .map_err(|_| anyhow!("invalid persisted device number"))?;
        let status = match (self.status.as_str(), self.borrower_id) {
            ("available", None) => DeviceStatus::Available,
            ("borrowed", Some(borrower)) => DeviceStatus::Borrowed {
                borrower: ActorId(
                    Uuid::parse_str(&borrower).context("invalid persisted borrower ID")?,
                ),
            },
            ("deregistered", None) => DeviceStatus::Deregistered,
            _ => {
                return Err(anyhow!(
                    "invalid persisted device status/borrower combination"
                ));
            }
        };
        let status = match (status, self.maintenance_work_order_id) {
            (DeviceStatus::Available, Some(id)) => DeviceStatus::UnderMaintenance {
                work_order_id: WorkOrderId(
                    Uuid::parse_str(&id).context("invalid persisted work order ID")?,
                ),
            },
            (status, None) => status,
            _ => return Err(anyhow!("invalid persisted maintenance/status combination")),
        };
        let device = Device::reconstitute(id, DeviceState { number, status })
            .unwrap_or_else(|never| match never {});
        Ok(DeviceSnapshot {
            device,
            version: self.version,
        })
    }
}
