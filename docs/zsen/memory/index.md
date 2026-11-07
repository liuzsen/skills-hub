# 项目地图

- [启动与验证](../../../README.md)：本地运行、检查、构建和部署命令。
- 前端入口：[src/App.tsx](../../../src/App.tsx)。主题由 next-themes 管理，健康检查请求与状态位于 [src/features/health/](../../../src/features/health/)。
- Rust 入口：[worker/src/lib.rs](../../../worker/src/lib.rs)。目前仅健康检查，没有业务层或存储绑定。
- 部署入口：[wrangler.jsonc](../../../wrangler.jsonc)。单 Worker 的 Rust API 与静态前端共用域名；本地 Vite 通过代理访问 Wrangler，见 [vite.config.ts](../../../vite.config.ts)。
