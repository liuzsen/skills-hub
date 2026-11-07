use crate::device::{ActorId, DeviceId};

/// A successful borrowing or return. Storage must retain append order per device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movement {
    pub device_id: DeviceId,
    pub actor_id: ActorId,
    pub kind: MovementKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementKind {
    Borrow,
    // 保留历史归还记录的存储判别值；归还行为与用例省略。
    Return,
}
