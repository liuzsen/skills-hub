# 后端 API

本地地址：`http://127.0.0.1:8787`。启动和管理员初始化见 [README](../README.md#本地后端)。前端目前使用 mock，不调用这些业务 API。

## 通用约定

- JSON 请求使用 `Content-Type: application/json`。认证、兑换、管理员请求体最多4096 bytes；内容上传上限见专项文档，均按实际流累计。
- 默认写请求必须携带精确 `Origin: http://127.0.0.1:8787`，且请求 URL 的 origin 也须匹配配置的 APP_ORIGIN。默认配置不接受 `localhost`、Vite 的5173端口或缺失 Origin。不提供跨域 CORS 接入。
- 登录身份来自 HttpOnly `hub_session` cookie；游客权限来自 `X-Guest-Token`，不使用 query/cookie 传游客凭证。
- 所有响应 `Cache-Control: no-store`。错误统一 `{ "error": "code" }`，不会返回内部 SQL、R2 key、凭证或 SDK 原文。
- 未知路由404 `not_found`；已知资源错误方法405 `method_not_allowed`，带 `Allow`。JSON格式错误400，超限413 `payload_too_large`，非JSON媒体类型415 `unsupported_media_type`，技术故障500 `internal_error`。
- 未配置的 APP_ENV 安全拒绝业务请求503；health 与未知路径不依赖绑定。默认构建不启用本地邮件；named local 同时要求 `local-mail` feature 和显式loopback APP_ORIGIN，允许主机仅 `127.0.0.1`、`localhost`、`[::1]`。非loopback HTTPS在local模式也拒绝503 `auth_unavailable`。Host/Origin请求头不能开启local，没有缺配置fallback或真实邮件投递。

## 覆盖开发端口

以8788为例，监听端口与允许的origin必须一起修改：

```sh
pnpm dev:api --port 8788 --var APP_ORIGIN:http://127.0.0.1:8788
BACKEND_ORIGIN=http://127.0.0.1:8788 pnpm backend:verify:auth
BACKEND_ORIGIN=http://127.0.0.1:8788 pnpm backend:verify:business
```

后两条在另一终端依次执行。本文curl示例的请求地址和 `Origin` header 也都替换为8788；只改其中一个会使写请求403。`BACKEND_ORIGIN` 默认 `http://127.0.0.1:8787`，只接受精确HTTP loopback origin，不改变Worker的APP_ORIGIN或CLI固定的local持久化目标。完整origin不带尾斜杠、路径、凭据、query或fragment。

## 路由索引

| 方法 | 路径                           | 权限 / 文档                                |
| ---- | ------------------------------ | ------------------------------------------ |
| GET  | `/api/health`                  | 公开；不访问存储                           |
| POST | `/api/auth/email/challenges`   | [发送验证码](#认证)                        |
| POST | `/api/auth/email/verify`       | [验证并登录](#认证)                        |
| GET  | `/api/me`                      | [当前账号](#认证)                          |
| POST | `/api/auth/logout`             | [退出](#认证)                              |
| POST | `/api/redemptions/guest`       | [游客一次领取](api-redemption.md)          |
| POST | `/api/redemptions/bind`        | [已登录账号绑定](api-redemption.md)        |
| POST | `/api/admin/redemption-codes`  | [管理员发码](api-redemption.md)            |
| POST | `/api/admin/capabilities`      | [管理员逐项授权](api-redemption.md)        |
| GET  | `/api/skills`                  | [公开目录](api-skills.md)                  |
| POST | `/api/skills`                  | [创建；upload_edit](api-skills.md)         |
| GET  | `/api/skills/{id}`             | [详情；对应 skill 查看权](api-skills.md)   |
| PUT  | `/api/skills/{id}`             | [更新；upload_edit](api-skills.md)         |
| GET  | `/api/skills/{id}/text?path=…` | [文本；对应 skill 查看权](api-skills.md)   |
| GET  | `/api/skills/{id}/download`    | [ZIP 流；对应 skill 下载权](api-skills.md) |

Health 返回 `{ "status": "ok", "service": "skills-worker", "runtime": "cloudflare-workers" }`。

## 认证

验证码有效600秒，同邮箱重发冷却60秒，最多5次错误；重发使旧码失效。正确验证码只可消费一次；首次验证自动注册，会话有效30天。邮箱先去首尾空白并转小写。

### 发送验证码

`POST /api/auth/email/challenges`

```json
{ "email": "user@example.com" }
```

成功202：

```json
{ "challenge_id": "…", "expires_at": 1790000600, "resend_after": 60 }
```

`expires_at` 为 Unix秒时间戳；`resend_after` 为再次发送前需等待的秒数（60），不是时间戳。400 `invalid_email`；429 `resend_cooldown` 带 `Retry-After: 60`；本地邮件未启用或缺pepper时503 `mail_unavailable`，配置无效也可能503 `auth_unavailable`。

本地验证码仅通过 `pnpm backend:mail user@example.com` 查看，不能通过 HTTP 查询；消费后删除，过期用 `pnpm backend:mail:purge` 清理。

### 验证并登录

`POST /api/auth/email/verify`

```json
{ "email": "user@example.com", "challenge_id": "…", "code": "123456" }
```

成功200 `{ "expires_at": 1792592000 }`，设置 `hub_session` cookie：HttpOnly、SameSite=Lax、Path=/；HTTPS带 Secure，仅显式本地HTTP分支省略 Secure。失败401 `challenge_rejected`，不区分过期、已消费、错误码、错误challenge。缺有效pepper时503 `auth_unavailable`。

### 当前账号

`GET /api/me`，携带登录cookie。成功200：

```json
{
  "id": "…",
  "email": "user@example.com",
  "is_admin": false,
  "capabilities": {
    "view_all": false,
    "download_all": false,
    "upload_edit": false
  }
}
```

缺失、错误、重复或已过期/吊销的session返回401 `unauthenticated`。管理员角色不隐式授予内容能力；三项能力独立授予，归并语义见专项文档。

### 退出

`POST /api/auth/logout`，携带cookie与精确Origin；无需JSON body。吊销当前有效session并清cookie，成功200 `{ "ok": true }`，重复退出仍成功。

## 手工登录

```sh
curl -i http://127.0.0.1:8787/api/auth/email/challenges \
  -H 'Origin: http://127.0.0.1:8787' -H 'Content-Type: application/json' \
  --data '{"email":"operator@example.com"}'
pnpm backend:mail operator@example.com
```

将响应中的challenge与CLI验证码填入登录请求。cookie文件存入忽略目录，勿提交：

```sh
mkdir -p .local
curl -i -c .local/backend-cookie.txt http://127.0.0.1:8787/api/auth/email/verify \
  -H 'Origin: http://127.0.0.1:8787' -H 'Content-Type: application/json' \
  --data '{"email":"operator@example.com","challenge_id":"…","code":"123456"}'
curl -b .local/backend-cookie.txt http://127.0.0.1:8787/api/me
```

继续使用[兑换/管理员 API](api-redemption.md)与[内容 API](api-skills.md)。本地管理员初始化由CLI显式执行，不自动提升首位账号。
