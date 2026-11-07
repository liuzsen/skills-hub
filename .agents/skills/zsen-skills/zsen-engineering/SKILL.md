---
name: zsen-engineering
description: zsen 的工程风格指南，agent 应当积极遵守。讨论需求、规划、实现、UI 设计、审查时使用。
---

本 Skill 中适用于当前任务的规范，应落实到方案、实现与验证中，而非仅供参考。

## 按任务选读

只读取当前任务涉及的文档；同时满足多个条件时组合读取。

- 讨论需求、规划方案，或实现中复杂度明显超出预期：[需求取舍](./references/scope.md)。
- 实现、重构、代码审查：[代码风格](./references/code-style.md)，[RUST](./references/rust-style.md)
- 规划、调整架构：[架构](./references/architecture.md)。
- 添加测试前必读：[测试与验证](./references/tests.md)。
- UI 规划、实现、审查：[UI](./references/ui.md)。

## 工作规范

- 项目根目录下的 `.local/` 已被 git ignored，可以建立子目录作为临时的工作目录
- 优先通过可靠机械工具维护其拥有的结构，工具缺失则先安装，禁止因此而手写，例如 `diesel migration generate`、`cargo add`、`pnpm add`
- 如果启动了 dev 服务器，在完成工作后关闭
