# 兑换与管理员 API

本契约供 API 调用方和内容模块接线使用。认证流程见认证模块文档；本文中的账号身份均来自 `hub_session`，目标 `account_id` 不是操作者身份。

## 请求约定

- 以下接口均为 `POST`，需 `Content-Type: application/json`，流式限制请求体为 4096 字节。未知 JSON 字段拒绝。
- `Origin` 必须与配置 `APP_ORIGIN` 完全相等，请求 URL 的 origin 也必须一致；缺失或不一致返回 `403 origin_forbidden`。
- 所有响应 `Cache-Control: no-store`。不匹配的方法返回 `405 method_not_allowed` 和 `Allow: POST`。
- 错误格式为 `{"error":"稳定错误码"}`。JSON/枚举/字段类型错误为 `400 invalid_request`；非 JSON 为 `415 unsupported_media_type`；超限为 `413 payload_too_large`；技术失败为 `500 internal_error`。
- 兑换码、游客 token 均为 32 字节 CSPRNG 随机值的小写 hex（64 字符）。数据库只存 SHA-256 摘要。

## 游客领取一次凭证

`POST /api/redemptions/guest`，公开接口，不要求登录。

```json
{ "code": "<64字符兑换码>" }
```

成功 `201`：

```json
{
  "token": "<64字符游客凭证>",
  "skill_id": "<skill id>",
  "expires_at": 1800000600
}
```

`expires_at` 为服务器 Unix 秒，等于领取时间 + 600。客户端应保存成功响应中的 token；响应丢失不能重新领取或恢复原 token。

- 每个码终身只有一次游客领取机会，不按浏览器、IP 或下载次数计数。
- 领取窗口内允许多次读取详情、文本和重试 ZIP 下载。
- 已领取、已绑定其他或当前账号、或不存在的码统一 `409 code_unavailable`；格式非法为 `400 invalid_code`。
- 窗口过期不会重置领取机会。领取后仍可永久绑定账号。

### 在内容请求中使用游客凭证

```http
X-Guest-Token: <领取响应中的token>
```

仅此 header 承载游客凭证；不使用 session cookie、query 参数或原始兑换码。格式非法 header 返回 `400`，由内容 HTTP 模块映射具体错误码。每次内容请求都查数据库，并用服务器当前时间判断 `guest_expires_at > now`；客户端提交的到期时间无效。

凭证只授权对应 skill。码绑定账号后，已发 token 保留原有有效期，但禁止再领取。已登录账号也可同时携带游客 header，内容权限取账号能力、绑定、有效游客凭证的并集。

## 永久绑定登录账号

`POST /api/redemptions/bind`，要求有效 session。

```json
{ "code": "<64字符兑换码>" }
```

成功 `200`：

```json
{ "skill_id": "<skill id>", "bound": true }
```

绑定提供对应 skill 长期详情和无限次最新 ZIP 下载权限，包含未来更新。游客是否领取、是否过期不影响绑定。

同一账号重复提交同一码返回相同成功结果。其他账号已绑定或不存在统一 `409 code_unavailable`；格式非法 `400 invalid_code`；未登录/会话过期 `401 unauthenticated`。

## 管理员发码

`POST /api/admin/redemption-codes`，操作者须通过 session 认证且具有独立管理员身份。

```json
{ "skill_id": "<已存在skill id>", "count": 2 }
```

每批 `count` 为 1–25 的整数。成功 `201`：

```json
{
  "skill_id": "<skill id>",
  "codes": [
    { "id": "<记录ID>", "code": "<明文兑换码>" },
    { "id": "<记录ID>", "code": "<明文兑换码>" }
  ]
}
```

**明文 code 只在此次创建响应返回，不能再次查询。** 响应丢失后再次发码是新批次，不是恢复操作；调用方应避免自动重试。

不存在的 skill 为 `404 skill_not_found`，不会插入任何码。数量超范围为 `400 invalid_count`；空或超过 256 字节的 skill ID 为 `400 invalid_skill_id`。

未登录/会话过期 `401 unauthenticated`；已登录但非管理员 `403 admin_required`。拥有三项内容能力不会自动成为管理员。

## 管理员逐项授予能力

`POST /api/admin/capabilities`，操作者须有管理员 session。

```json
{ "account_id": "<已注册账号的64字符ID>", "capability": "view_all" }
```

`capability` 三选一：

| 值             | 权限                                              |
| -------------- | ------------------------------------------------- |
| `view_all`     | 全部 skill 的详情、文件树、README、文本；不含 ZIP |
| `download_all` | 全部 skill 的 ZIP，包含全部详情                   |
| `upload_edit`  | 创建/编辑全部 skill，包含全部详情；不含 ZIP       |

成功 `200 {"ok":true}`。重复授予幂等；每次只置一项标志，保留其他标志，不创建管理员身份、不创建账号。本接口不提供撤销。

账号 ID 格式非法为 `400 invalid_account_id`；账号未注册为 `404 account_not_found`；无会话 `401 unauthenticated`；非管理员 `403 admin_required`。

## 内容模块授权接线

共享 port：`hub_application::redemption::access::SkillAccess`。

```rust
access.authorize(&actor, guest_token_hash.as_ref(), &skill_id, operation).await
```

- `Actor` 复用 `hub_domain::account::Actor`，由 HTTP session 认证构造。
- `SkillOperation::Read` 用于详情/树/README/文本；`Download` 用于 ZIP。
- HTTP 调用 `hub_http::redemption::guest_token_hash(&request)`，得到已摘要的 `Option<GuestTokenHash>`。请求不得直接提交摘要代替原 token。
- 内容用例先 authorize 成功，再读取受限元数据/R2。`AccessDenied::Forbidden` 不授予任何受限内容；技术错误不是业务拒绝。
- 组合根注入 `CheckSkillAccess { store: RedemptionStore::new(db), clock }`。适配器提供事实；账号能力归并由 domain/application 统一处理。授权成功不保证 skill 存在，存在性由内容模块检查。

## 并发与集成验收

主 Agent 使用真实本地 D1/HTTP 验证：

1. 同码并发游客领取恰一个 `201`，其他 `409`；到期后不能重新领取。
2. 两账号并发绑定恰一个 owner；同 owner 重试 `200`，其他账号 `409`。
3. 领取/绑定竞争：绑定先提交则领取失败；领取先提交则两者可成功，旧 token 到原期限仍有效。
4. 游客 token 无法读取其他 skill；临界 `expires_at == now` 已无效；游客过期后仍可绑定。
5. 逐项授权分别验证 Read/Download/上传编辑；管理员但零能力的账号不能直接查看或下载未绑定内容。
6. 缺 Origin、相近但不完全一致的 Origin、任意请求体 actor ID 均不能完成写入；不存在账号/skill 不产生副作用。
7. D1 中不出现明文 code/token，日志和错误响应不泄露二者；重启后绑定、游客使用记录、能力仍持久。

本模块交付仅进行了 Rust AST 与 SQL prepare 语法检查；上述运行时验收由主 Agent 集中执行。
