# 项目地图

- [启动与验证](../../../README.md)：本地运行、检查、构建和部署命令。
- 前端入口：[src/App.tsx](../../../src/App.tsx)，当前是纯本地 Skills 用户端原型，不请求健康检查或业务 API。目录、权限与兑换状态、登录弹窗、文件阅读分别位于 [src/features/skills/](../../../src/features/skills/)，状态入口为 [mock-state.ts](../../../src/features/skills/mock-state.ts)。刷新清空 mock 授权与兑换记录；页面底部折叠工具可切换场景。主题由 next-themes 管理。
- Rust 入口：[worker/src/lib.rs](../../../worker/src/lib.rs)。目前仅健康检查，没有业务层或存储绑定。
- 部署入口：[wrangler.jsonc](../../../wrangler.jsonc)。单 Worker 的 Rust API 与静态前端共用域名；本地 Vite 通过代理访问 Wrangler，见 [vite.config.ts](../../../vite.config.ts)。
