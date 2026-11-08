# Skills Hub

React 前端与 Rust Cloudflare Workers 后端。后端提供邮箱验证码认证、公开目录、受限详情/文本/ZIP、兑换绑定与管理员上传授权；前端仍是独立 mock 原型，未接业务 API。

## 环境准备

需要 Node.js 24.13+、pnpm 11.1.1、Rust stable：

```sh
corepack enable
corepack prepare pnpm@11.1.1 --activate
pnpm install --frozen-lockfile
rustup target add wasm32-unknown-unknown
rustup component add rustfmt clippy
cargo install worker-build --version 0.8.7 --locked
```

## 本地后端

```sh
pnpm backend:init   # 生成忽略的本地 AUTH_PEPPER，应用0001/0002/0003迁移
pnpm dev:api        # http://127.0.0.1:8787
```

无须 Cloudflare 登录。所有数据库操作固定 named `local` 环境、`--local` 与 `.wrangler/state`，D1/R2 绑定均 `remote=false`。保留 `.dev.vars.local` 和 `.wrangler/state` 可在重启后继续使用；不要公开 pepper 或本地数据库。按 Ctrl+C 停止服务。

```sh
curl http://127.0.0.1:8787/api/health
# {"status":"ok","service":"skills-worker","runtime":"cloudflare-workers"}
```

API 契约与登录示例见 [docs/api.md](docs/api.md)，专项见[兑换/管理员](docs/api-redemption.md)、[内容发布与读取](docs/api-skills.md)。默认写请求使用精确 `Origin: http://127.0.0.1:8787`；此配置下不接受 localhost、Vite Origin 或缺失 Origin。

```sh
pnpm backend:migrate                 # 拉取新迁移后执行
pnpm backend:mail user@example.com    # CLI读取该邮箱未过期验证码；无HTTP调试接口
pnpm backend:mail:purge               # 清理过期明文outbox
pnpm backend:verify:auth              # dev:api运行时，验证真实D1/HTTP认证与并发
pnpm backend:verify:business          # dev:api运行时，验证内容/兑换/授权与ZIP
```

默认构建不启用 `local-mail`，默认 APP_ENV/MAIL_TRANSPORT 均 disabled，业务请求安全拒绝。仅 `local-mail` feature、APP_ENV=local、MAIL_TRANSPORT=d1-outbox、有效 pepper、显式 loopback APP_ORIGIN 与请求精确匹配同时满足时才写本地 outbox；不发送真实邮件，不把验证码写日志。其他令牌仅存摘要，认证表验证码仅存 HMAC。所有API响应 no-store。

### 覆盖本地端口

8787被占用时，可保留原服务，另选端口并同时覆盖 APP_ORIGIN：

```sh
pnpm dev:api --port 8788 --var APP_ORIGIN:http://127.0.0.1:8788
# 另一终端
BACKEND_ORIGIN=http://127.0.0.1:8788 pnpm backend:verify:auth
BACKEND_ORIGIN=http://127.0.0.1:8788 pnpm backend:verify:business
```

请求地址和 `Origin` header 均改为 `http://127.0.0.1:8788`。仅改监听端口不会自动改 APP_ORIGIN；`BACKEND_ORIGIN` 只选择验证脚本的目标，不修改服务配置或本地D1/R2目标。两份脚本默认8787，只接受精确HTTP loopback origin。

local 模式的 APP_ORIGIN 必须是 `127.0.0.1`、`localhost` 或 `[::1]` 的精确 HTTP/HTTPS origin，不带路径、凭据、query或fragment；非loopback即使HTTPS也拒绝。Host/Origin请求头不能单独开启local，无缺配置fallback。默认生产HTTPS策略仍要求Secure cookie。

### 初始化管理员

先用指定邮箱完成验证码登录，再执行：

```sh
pnpm backend:admin operator@example.com
# 按需显式授予各项能力
pnpm backend:admin operator@example.com --view-all --download-all --upload-edit
```

缺失账号非零退出，不创建账号；首位注册者不自动提权。管理员本身没有隐式内容能力。重复初始化幂等，仅增加明确指定能力。

### 前端原型

`pnpm dev` 同时启动 Vite 与上述本地 Worker。打开 <http://localhost:5173>，当前页面完全使用本地 mock。Vite 的 `/api` 代理不改变后端精确 Origin 策略；业务 API 验证直接请求8787，不以页面 mock 验证后端权限。

## 检查与构建

```sh
cargo check --manifest-path worker/Cargo.toml --workspace --target wasm32-unknown-unknown --locked
cargo check --manifest-path worker/Cargo.toml --workspace --all-features --target wasm32-unknown-unknown --locked
pnpm check                 # TypeScript、Oxlint、格式与Rust Clippy
pnpm format                # 整理格式
pnpm build:worker          # 默认feature，禁用本地邮件
pnpm build:worker:local    # 显式启用local-mail
pnpm build                # 前端dist/ + 默认Worker
pnpm preview              # 本地单Worker静态资源与API；启动时构建local Worker
```

`pnpm preview` 前需有前端 `dist/`；不要使用 `vite preview` 验证API。切换默认/local模式必须重新构建，不复用上一份Wasm。

## 配置与源码入口

- [worker/src/lib.rs](worker/src/lib.rs)：组合根，按路由装配请求级绑定；[downloads.rs](worker/src/downloads.rs)：已授权 ZIP 的流式交接。
- `worker/crates/hub-domain/`：账号、skill不变量与兑换权限；`hub-application/`：固定rev lolibaso UseCase与存储ports。
- `worker/crates/hub-http/`：HTTP DTO、方法/Origin/请求体边界；不依赖具体D1/R2。
- `worker/crates/hub-d1/`、`hub-r2/`：元数据与不可变文件/ZIP适配器。迁移位于 `worker/crates/hub-d1/migrations/`。
- [wrangler.jsonc](wrangler.jsonc)：`/api`、`/api/*` 优先进入Rust，其余请求由静态资源提供SPA；开发命令用 `public/` 覆盖静态目录，不依赖先构建前端。
- [src/App.tsx](src/App.tsx)、`src/features/skills/`：前端mock；[vite.config.ts](vite.config.ts)：本地前端与API代理。

部署目标为 Cloudflare，但当前绑定仅配置本地模拟，没有远程资源ID或真实邮件投递。线上配置与部署须另行授权；本交付不执行部署。
