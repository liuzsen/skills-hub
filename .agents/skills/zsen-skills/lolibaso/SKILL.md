---
name: lolibaso
description: lolibaso 是一个轻量级的 rust clean architecture toolkit，可任意组合能力。
---

# 使用 lolibaso

lolibaso 提供通用类型及辅助宏；数据库访问、事务执行和 Web 框架适配由应用实现。

- 使用 [SOURCE.json](references/source/SOURCE.json) 记录的 commit 作为 Git 依赖的 `rev`，按需启用 feature。
- 过程宏用法看示例：[Entity](references/source/examples/entity.rs)、[注入宏](references/source/examples/inject.rs)、[HttpErrorCode](references/source/examples/http_error.rs)。
- 其他 API 与生成细节看[随附源码快照](references/source/)。
