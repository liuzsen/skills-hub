# 项目地图

- [README](../../../README.md)：环境准备、本地D1/R2初始化、显式管理员、端口覆盖与检查/构建命令。[API入口](../../api.md)描述认证，链接[兑换与管理员](../../api-redemption.md)、[内容](../../api-skills.md)专项。
- 前端入口：[src/App.tsx](../../../src/App.tsx)。当前是纯本地 Skills 用户端原型，不调用健康检查或业务 API；目录、权限与兑换、登录弹窗、文件阅读在 [src/features/skills/](../../../src/features/skills/)，状态入口 [mock-state.ts](../../../src/features/skills/mock-state.ts)。刷新清空mock状态，页面底部工具切换场景，主题由next-themes管理。
- Rust组合根：[worker/src/lib.rs](../../../worker/src/lib.rs)按路由构造请求级绑定，连接认证、兑换、管理员、内容API并保留health；[downloads.rs](../../../worker/src/downloads.rs)将已授权的R2 ZIP流交给HTTP，不缓冲整包。
- Cargo workspace位于 [worker/Cargo.toml](../../../worker/Cargo.toml)：`hub-domain`定义账号/skill/兑换不变量，`hub-application`提供lolibaso UseCase与ports，`hub-http`处理DTO/身份/Origin，`hub-d1`与`hub-r2`实现持久化。HTTP不依赖具体D1/R2。迁移入口是 [hub-d1/migrations/](../../../worker/crates/hub-d1/migrations/)。
- [wrangler.jsonc](../../../wrangler.jsonc)：Rust API与静态前端共用Worker，`/api`优先进入Rust；本地Vite代理见 [vite.config.ts](../../../vite.config.ts)，前端尚未接入业务。named local使用`DB`与私有`SKILLS`本地绑定，持久化在忽略的`.wrangler/state`；没有配置远程资源ID或真实邮件。
- 本地操作入口：[scripts/backend-local.mjs](../../../scripts/backend-local.mjs)。`pnpm backend:init`准备忽略的`.dev.vars.local`与迁移；`pnpm dev:api`启动服务；mail/admin命令固定local数据库。outbox仅CLI可读，`local-mail` feature、APP_ENV=local、transport/pepper和精确loopback APP_ORIGIN共同启用本地投递；检查在 [HttpPolicy](../../../worker/crates/hub-http/src/policy.rs)与组合根。
- 真实HTTP验证入口：[认证smoke](../../../scripts/backend-auth-smoke.mjs)、[业务smoke](../../../scripts/backend-business-smoke.mjs)，分别由`backend:verify:auth`、`backend:verify:business`运行，前置条件是已初始化并启动local Worker。两者支持`BACKEND_ORIGIN`，默认`http://127.0.0.1:8787`，仅接受精确HTTP loopback origin；端口覆盖需同步服务APP_ORIGIN，步骤见README。
