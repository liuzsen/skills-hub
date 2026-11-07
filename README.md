# Skills Hub

React + Rust Cloudflare Workers 项目底座，目前仅包含基础布局、主题切换和真实后端健康检查。

## 本地启动

需要 Node.js 24.13+、pnpm 11.1.1、Rust stable（含 rustfmt、Clippy）。首次准备：

```sh
corepack enable
corepack prepare pnpm@11.1.1 --activate
pnpm install --frozen-lockfile
rustup target add wasm32-unknown-unknown
rustup component add rustfmt clippy
cargo install worker-build --version 0.8.7 --locked
```

```sh
pnpm dev
```

打开 <http://localhost:5173>。Vite 将 `/api` 代理到 `127.0.0.1:8787` 的 Rust Worker。首次启动需下载 Rust/Wasm 工具依赖；无须 Cloudflare 登录或配置密钥。按 `Ctrl+C` 同时关闭两个服务。

```sh
curl http://127.0.0.1:5173/api/health
# {"status":"ok","service":"skills-worker","runtime":"cloudflare-workers"}
```

页面可切换亮暗主题；首次跟随系统，手动选择后保存在浏览器中。连接失败时确认 Worker 终端没有报错，再点击「重新检查」。

## 检查与构建

```sh
pnpm check           # TypeScript、Oxlint、格式和 Rust Clippy
pnpm format          # 自动整理格式
pnpm build           # 前端 dist/ + Rust Worker 的 JS/Wasm
pnpm preview         # 在 8787 端口验证单 Worker 静态资源与 API
pnpm deploy:dry-run  # 构建并检查部署包，不上传
```

`pnpm preview` 前先运行 `pnpm build`。不要使用 `vite preview` 验证 API：它不包含 Rust Worker。

## 部署

`wrangler.jsonc` 配置一个 `skills-hub` Worker：`/api`、`/api/*` 优先交给 Rust，其他路径由 Workers Static Assets 提供 `dist/`，未命中时回退 SPA。没有存储绑定。开发命令用 `public/` 覆盖静态目录，前端由 Vite 提供，因此首次开发不依赖 `dist/`。

确认 Cloudflare 账号和 Worker 名称后，登录并部署：

```sh
pnpm exec wrangler login
pnpm exec wrangler whoami
pnpm deploy
```

部署命令会构建前端，Wrangler 的自定义构建步骤编译 Rust。默认使用 Workers 地址；自定义域名在确认账号和域名配置后另行接入。本仓库尚未执行线上部署。

## 项目入口

- `src/App.tsx`：基础布局；`src/features/health/`：健康检查状态与请求校验。
- `src/components/ui/`、`components.json`：官方 shadcn CLI 生成的组件与配置。
- `worker/src/lib.rs`：Rust HTTP 入口，`GET /api/health` 返回 JSON；其他方法返回 405，未知 API 返回 404。
- `vite.config.ts`：Tailwind 与本地 API 代理；`wrangler.jsonc`：构建、静态路由和可观测性。
- `.agents/`、`skills-lock.json`：原有 skills 文件。
