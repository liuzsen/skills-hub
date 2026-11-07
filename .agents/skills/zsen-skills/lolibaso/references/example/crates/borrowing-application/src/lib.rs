pub mod borrow_device;
pub mod register;
// 省略查询、归还、注销及维修协作用例的模块声明；只收录借用与登记差异。

use borrowing_domain::{device::Device, movement::Movement};
use lolibaso::repository::TransactionalRepository;

/// Shared borrowing/return port. The transaction callback receives this same context.
///
/// Implementations must atomically commit device updates and appended movements,
/// rolling both back on either business refusal or technical error. An update must
/// compare the snapshot's version and report StaleState instead of overwriting it.
/// Database locks and infrastructure failures remain technical errors.
pub trait CirculationRepository: TransactionalRepository<Device> {
    /// Append in this context's active transaction, never on a separate connection.
    fn append_movement(
        &mut self,
        movement: Movement,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}
