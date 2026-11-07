# Rust 专项

## 四层架构

持续维护的服务、核心应用采用整洁架构，不因当前规模小而混合已存在的职责：

```text
Domain ← Application ← Adapter ← Infrastructure
```

源码依赖由外向内。需要外层能力时，由消费方以自身语言定义 port，外层实现。技术库不是架构层，调用路径不必经过每层。

- **Domain**：业务判断、不变量与合法变化，不组织应用流程。
- **Application**：以 UseCase 组织业务意图，决定输入输出、业务失败、事务与副作用。
- **Adapter**：协议形状与转换。入口将 HTTP、CLI、Message 等转为应用语言；出口实现消费方能力契约，将外部语义转为项目语义。独立治理的 Contract 可提取为本层 crate，不新增一层。
- **Infrastructure**：资源、运行机制与组装启动。连接池、runtime、日志等通用机制属于 None；具体实现选择、Protocol Host、Composition 与入口属于 System。

Adapter 可使用实现协议所需的库，但不管理通用运行机制的生命周期。比如 Adapter 与 Infrastructure 都依赖 diesel-async，但两者的职责依然分明，需要的 diesel-async 能力不同。

## UseCase

从业务需求出发，将 App 拆分为多个可独立理解的 UseCase，再推导 Domain 定义、所需 port，延后实现技术机制，比如数据库 Adapter。

每个 UseCase 一个文件模块，Input、Output、Error、ports 与流程放在一起。

多个 UseCase 可按业务焦点归入目录，`lib.rs`、`mod.rs` 除模块声明与导出外，也可定义模块共用的核心类型、trait 和少量逻辑，或集中到 ports.rs。

共享规则与能力契约留在所属业务组，port 表达用例实际需要的最小能力，避免定义万能 trait。

UseCase 不互相调用；共用流程提取后留在 Application，由调用方决定 Transaction 生命周期。

业务写命令必须经过 Application；查询中的授权、租户隔离、可见性与一致性规则由 Application/Domain 承载。简单查询可绕过 Application。

## crate 边界

不同层级使用 crate 隔离，比如 Domain、Application、各协议 Adapter 与独立 Infrastructure 分为不同 crate。仅当 `职责局部 + 稳定无需扩展 + 提取成本明显不相称` 时，保留为 module，例如 executable 中的日志模块。

单个 member context 示例：

```text
apps/web-server/                 # System × Infrastructure：入口与组装
src-tauri/  # tauri 是 app，为了保持 tauri 项目结构允许独立
crates/member/
  member-domain/              # Member × Domain
  member-application/         # Member × Application：UseCase、出口 port
  member-http/                # Member × Adapter：HTTP 入口转换
  member-postgres/            # Member × Adapter：持久化转换、资源 port
  postgres-runtime/           # None × Infrastructure：连接池生命周期
```

## 库

使用 lolibaso clean architecture toolkit。
