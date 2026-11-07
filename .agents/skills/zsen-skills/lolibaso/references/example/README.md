# 从实际项目学习 lolibaso

本例用设备借用与维修展示目录组织和代码风格。先看依赖方向，再按任务选读，不必通读全部文件。

这里是**经过裁剪的教学摘录，不是可编译的 workspace**：保留借用设备主链路与登记差异，省略同构用例、部分业务方法、迁移、Cargo 清单及运行教程。局部省略用注释或 `...` 标记；`...` 是教学省略符，不是 Rust 语法。需要确认 toolkit API 时看[源码快照](../source/)，依赖 revision 以其中的 [SOURCE.json](../source/SOURCE.json) 为准。

## 目录与依赖方向

上游项目按领域、层和具体适配器拆 crate；下面是布局概览，不表示每个目录都已收录：

```text
apps/
  borrowing-server/          # 借用服务：启动、配置、监听
  maintenance-server/        # 维修服务：独立进程与恢复 worker
  monolith/                  # 单体：组合两个领域与进程内协作
crates/
  borrowing-domain/          # 实体、值对象、不变量
  borrowing-application/     # 用例及其所需的 port
  borrowing-http/            # HTTP DTO、身份、handler、错误映射
  borrowing-sqlite/          # 行映射、仓储、事务
  borrowing-postgres/        # 同一仓储 port 的另一实现
  maintenance-domain/        # 维修工单状态
  maintenance-application/   # 维修用例、消费方协作 port
  maintenance-http/         # 维修 HTTP 入口
  maintenance-sqlite/       # 工单持久化与待办
  maintenance-borrowing/    # 进程内协作 Adapter
  maintenance-borrowing-http/ # HTTP 协作 Adapter
  example-runtime/          # 具体资源、用例工厂、共享启动与 worker
```

```text
HTTP / 数据库 Adapter → Application → Domain
协作 Adapter → 消费方 port + 提供方 Application 或 HTTP 协议
app → 具体 Adapter + example-runtime（资源与用例组装）
```

Domain / Application 不依赖 HTTP、数据库或运行时。HTTP 不依赖 SQLite；组合根才知道具体实现。`example-runtime` 是多个部署入口的组装复用，不是业务层，也不是每个项目都必须有的 crate。

## 按任务选读

| 要学习什么 | 阅读入口与关注点 |
| --- | --- |
| 实体与恢复 | [device.rs](crates/borrowing-domain/src/device.rs)：`Entity`、类型化 ID、值对象和合法状态变化；[device_row.rs](crates/borrowing-sqlite/src/device_row.rs)：先校验持久化值，再 `reconstitute` |
| 用例与 port | [Application lib.rs](crates/borrowing-application/src/lib.rs) → [borrow_device.rs](crates/borrowing-application/src/borrow_device.rs)：`UseCase` 按值持有泛型 port，设备与[借还记录](crates/borrowing-domain/src/movement.rs)同事务写入 |
| 登记的不同之处 | [register.rs](crates/borrowing-application/src/register.rs)：注入 `IdGenerator`，将编号占用映射为业务拒绝，ID 碰撞仍是技术失败 |
| 数据库实现 | [SQLite lib.rs](crates/borrowing-sqlite/src/lib.rs) → [repository.rs](crates/borrowing-sqlite/src/repository.rs)：拥有型连接、Snapshot/CAS、事务及取消后的连接处置 |
| HTTP 边界 | [borrow.rs](crates/borrowing-http/src/devices/borrow.rs) → [error.rs](crates/borrowing-http/src/error.rs) → [router.rs](crates/borrowing-http/src/router.rs)：handler 局部 Factory、Input 构造、错误翻译与协议响应 |
| 注入与组合根 | [factory.rs](crates/example-runtime/src/borrowing/factory.rs) → [sqlite_database.rs](crates/example-runtime/src/borrowing/sqlite_database.rs) → [main.rs](apps/borrowing-server/src/main.rs)：每次获取独占连接，由 `TypeBuilder` 递归构造 owned Case |
| 跨领域协作 | [进程内与 HTTP 差异案例](topics/collaboration.md)：消费方 port → Adapter 的 Factory 与调用 → 组合根绑定；HTTP 只作边界翻译对照，不展开恢复协议 |

**主线阅读顺序**：实体 → Application 契约 → 借用用例 → 仓储 → HTTP handler → Factory → main。它们是同一条执行链，不是互不关联的 API 小样。

## 从例子中提取约定

- Application 的外层 `anyhow::Result` 表达技术失败，内层业务 enum 表达拒绝；业务错误不必实现标准 `Error`，显示与协议转换由 Adapter 决定。
- 事务只在 `Ok(Ok(_))` 时提交；内层拒绝和外层失败都回滚。事务中的设备写入与历史追加使用同一仓储上下文。
- 数据库版本属于 Snapshot，不进入实体。CAS 未更新到行返回 `StaleState`，用例决定如何表达并发冲突。
- handler 提取入口身份、构造 Input，取得 owned Case 后执行；具体资源获取与 DI 留给组装方。trait 和 Factory 可按 handler 需求局部定义。
- 代表实现保留 import 和完整签名：依赖来源、泛型、生命周期与异步边界也是风格的一部分。

设备编号规则、借用状态、HTTP 路径及 `X-Dev-User` 都是示例选择；开发身份不是生产认证。多 crate 拆分、两个数据库文件、是否采用恢复 worker，应按实际项目边界选择，不是使用 lolibaso 的前置条件。

## 适用边界

SQLite 示例使用 Diesel 的同步连接异步 wrapper，依赖 Tokio multi-thread runtime；它不是原生异步 driver。事务取消或控制失败会丢弃连接；COMMIT 已开始后取消，提交结果可能未知，不能据取消无条件重试。PostgreSQL 的原生异步连接与事务控制细节未重复收录，不能将 SQLite 的锁策略直接套用到其他数据库。

上游测试按外部边界组织：数据库 crate 的 `tests/integration.rs` 注册 `tests/database/` 子模块，以真实临时数据库和独立连接检查提交、回滚与竞争；app 测试通过真实 binary/TCP 检查协议及重启持久化。本摘录未收录测试夹具和故障矩阵，也没有提供可运行的验证命令。

lolibaso 的其他可选能力未必在本业务中出现；需要实体 patch/updater 等用法时，另查源码快照与其中的宏示例。
