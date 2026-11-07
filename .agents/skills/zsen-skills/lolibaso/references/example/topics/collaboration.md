# 跨领域协作：port、Adapter 与组合根

本页展示两个业务模块如何连接：消费方定义 port，Adapter 调用提供方用例，组合根选择实现。以下按上游路径摘录；`...` 表示省略同构代码，不是 Rust 语法。

## 消费方定义 port

`crates/maintenance-application/src/lib.rs`：维修只声明自己需要的能力，不依赖借用领域的用例或数据库。

```rust
use lolibaso::biz::BizResult;
use maintenance_domain::repair_order::{DeviceId, OccupationRefusal, RepairOrderId};

pub trait DeviceMaintenancePort: Send {
    fn occupy(
        &mut self,
        order_id: RepairOrderId,
        device_id: DeviceId,
    ) -> impl Future<Output = BizResult<(), OccupationRefusal>> + Send;

    fn release(
        &mut self,
        order_id: RepairOrderId,
        device_id: DeviceId,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}
```

本例契约还要求：以原订单/设备对重试，占用决定（含拒绝）可重放，先释放后到达的占用不能复活占用；配对错误属于技术失败。这里保留契约，不展开业务的恢复协议。

## 进程内 Adapter：调用对方用例

`crates/maintenance-borrowing/src/lib.rs`：Factory 每次提供 owned 用例。Adapter 只翻译 ID 与结果，不打开数据库，也不读取对方表。省略释放相关的 Factory 成员、方法实现及同构拒绝映射。

```rust
use borrowing_application::{maintenance::OccupationRejection, occupy_for_maintenance};
use borrowing_domain::device::{DeviceId as BorrowingDeviceId, WorkOrderId};
use lolibaso::biz::{BizResult, UseCase};
use maintenance_application::DeviceMaintenancePort;
use maintenance_domain::repair_order::{DeviceId, OccupationRefusal, RepairOrderId};

pub trait Factory: Clone + Send + Sync + 'static {
    type Occupy: UseCase<
        Input = occupy_for_maintenance::Input,
        Output = (),
        Error = occupy_for_maintenance::Error,
    >;

    fn create_occupy(&self) -> impl Future<Output = anyhow::Result<Self::Occupy>> + Send;
    ...
}

#[derive(Clone)]
pub struct InProcess<F> {
    pub factory: F,
}

impl<F: Factory> DeviceMaintenancePort for InProcess<F> {
    async fn occupy(
        &mut self,
        order_id: RepairOrderId,
        device_id: DeviceId,
    ) -> BizResult<(), OccupationRefusal> {
        let result = self
            .factory
            .create_occupy()
            .await?
            .execute(occupy_for_maintenance::Input {
                work_order_id: WorkOrderId(order_id.0),
                device_id: BorrowingDeviceId(device_id.0),
            })
            .await?;
        match result {
            Ok(()) => Ok(Ok(())),
            Err(occupy_for_maintenance::Error::WorkOrderDeviceMismatch) => {
                anyhow::bail!(
                    "borrowing occupation pair mismatch: order={}, device={}",
                    order_id.0,
                    device_id.0
                )
            }
            Err(occupy_for_maintenance::Error::Rejected(reason)) => Ok(Err(match reason {
                OccupationRejection::NotFound => OccupationRefusal::DeviceNotFound,
                ...
            })),
        }
    }
    ...
}
```

`await?` 传播技术失败；提供方的业务拒绝翻译为消费方词汇。配对错误虽然由提供方用例的内层 enum 返回，但对消费方是 Adapter 使用错误，因此转为外层技术失败，不能与普通拒绝一起机械映射。

## 组合根：配置资源并连接双方

`crates/example-runtime/src/borrowing/factory.rs`：在已有[借用工厂](../crates/example-runtime/src/borrowing/factory.rs)上实现协作 Factory。具体连接与 `TypeBuilder` 留在这里，不进入 Adapter。

```rust
use borrowing_application::occupy_for_maintenance;
// Context、SqliteRepository、TypeBuilder 与 UseCaseFactory 定义见借用工厂。

impl maintenance_borrowing::Factory for UseCaseFactory {
    type Occupy = occupy_for_maintenance::OccupyForMaintenance<SqliteRepository>;

    async fn create_occupy(&self) -> anyhow::Result<Self::Occupy> {
        TypeBuilder::new()
            .add_instance(self.database.connect().await?)
            .build_async::<Self::Occupy, _>()
            .await
            .context("creating occupy for maintenance use case")
    }
    ...
}
```

`apps/monolith/src/main.rs`：把借用工厂交给 Adapter，再把 Adapter 交给维修工厂。省略配置、路径检查、HTTP 和 worker 启动。

```rust
use example_runtime::{borrowing, maintenance};
use maintenance_borrowing::InProcess;

let borrowing = borrowing::UseCaseFactory {
    database: borrowing::SqliteDatabase::initialize(&borrowing_path).await?,
};
let maintenance = maintenance::UseCaseFactory {
    database: maintenance::SqliteDatabase::initialize(&maintenance_path).await?,
    devices: InProcess {
        factory: borrowing.clone(),
    },
};
```

`crates/example-runtime/src/maintenance/factory.rs` 的 `UseCaseFactory<P>` 持有配置好的 `devices: P`。其 `advance()` 先用 `TypeBuilder` 构造独占仓储，再显式绑定该运行时 port，而非让 DI 递归构造它：

```rust
// repository 已由 TypeBuilder 从本次取得的独占连接构造。
Ok(advance_pending::AdvancePending {
    repository,
    devices: self.devices.clone(),
})
```

## HTTP 实现：只替换边界翻译

`crates/maintenance-borrowing-http/src/lib.rs` 的 `HttpBorrowing` 实现同一个 port。它发送 wire DTO 而非调用提供方用例；这里只保留响应分类，省略请求构造和同构拒绝映射。

```rust
match response.status() {
    StatusCode::OK => {
        decode::<wire::SuccessResponse>(response).await?;
        Ok(Ok(()))
    }
    StatusCode::CONFLICT => {
        let refusal = decode::<wire::OccupationRefusalResponse>(response).await?;
        Ok(Err(match refusal.error {
            wire::OccupationRefusalCode::DeviceNotFound => {
                OccupationRefusal::DeviceNotFound
            }
            ...
        }))
    }
    status => anyhow::bail!("unexpected borrowing occupation HTTP status: {status}"),
}
```

网络失败、非法响应和意外状态仍是技术错误，不包装成业务拒绝。组合根可换用 `HttpBorrowing`，消费方 port 不变。

进程内或 HTTP 都不意味着跨领域共享事务；本例契约禁止在维修本地事务内调用协作 port。是否需要持久恢复及如何重试，是应用协议的设计，不是这种 Adapter 结构的前置条件。
