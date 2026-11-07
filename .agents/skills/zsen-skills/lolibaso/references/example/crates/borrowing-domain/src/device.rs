use std::convert::Infallible;

use lolibaso::entity::{Entity, EntityId, Reconstitute};
use uuid::Uuid;

use crate::movement::{Movement, MovementKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceId(pub Uuid);

impl EntityId for DeviceId {}

/// Identity supplied by a trusted caller; this domain does not manage accounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActorId(pub Uuid);

/// 仅保留维修身份以重建已有状态；维修行为和协作协议不在本摘录中。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkOrderId(pub Uuid);

/// Case-sensitive equipment number. Deregistration does not release the number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceNumber(String);

impl DeviceNumber {
    pub fn parse(value: String) -> Result<Self, InvalidNumber> {
        if value.is_empty() || value.trim() != value {
            return Err(InvalidNumber);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
    // 省略未使用的 into_string 转换。
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidNumber;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStatus {
    Available,
    Borrowed { borrower: ActorId },
    UnderMaintenance { work_order_id: WorkOrderId },
    Deregistered,
}

#[derive(Entity, Debug, Clone)]
pub struct Device {
    id: DeviceId,
    number: DeviceNumber,
    status: DeviceStatus,
}

// Persistence adapters parse raw numbers before constructing that state.
impl Reconstitute for Device {
    type State = DeviceState;
    type Error = Infallible;

    fn reconstitute(id: DeviceId, state: DeviceState) -> Result<Self, Self::Error> {
        Ok(state.into_entity_unchecked(id))
    }
}

impl Device {
    pub fn register(id: DeviceId, number: String) -> Result<Self, InvalidNumber> {
        Ok(Self {
            id,
            number: DeviceNumber::parse(number)?,
            status: DeviceStatus::Available,
        })
    }

    pub fn borrow(&mut self, actor: ActorId) -> Result<Movement, BorrowError> {
        match self.status {
            DeviceStatus::Available => {}
            DeviceStatus::Borrowed { .. } => return Err(BorrowError::AlreadyBorrowed),
            DeviceStatus::UnderMaintenance { .. } => return Err(BorrowError::UnderMaintenance),
            DeviceStatus::Deregistered => return Err(BorrowError::Deregistered),
        }
        self.status = DeviceStatus::Borrowed { borrower: actor };
        Ok(Movement {
            device_id: self.id,
            actor_id: actor,
            kind: MovementKind::Borrow,
        })
    }

    // 省略归还、注销、维修占用/释放及其专用错误；借用仍完整判断这些既有状态。
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorrowError {
    AlreadyBorrowed,
    UnderMaintenance,
    Deregistered,
}
