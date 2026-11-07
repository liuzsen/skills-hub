use borrowing_domain::device::DeviceId;
use lolibaso::entity::IdGenerator;
use uuid::Uuid;

#[derive(lolibaso::inject::Inject)]
pub struct DeviceIds;

impl IdGenerator for DeviceIds {
    type Id = DeviceId;

    fn generate(&mut self) -> DeviceId {
        DeviceId(Uuid::new_v4())
    }
}
