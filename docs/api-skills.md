# Skills 内容 API

本页供后端接线和本地 API 验证使用。接口不接入前端，不提供历史版本选择。路径前缀为 `/api/skills`；JSON 字段使用 camelCase。

## 权限与请求约定

公开列表无需身份。详情、文件清单、README、文本和 ZIP 必须先授权：

| 身份/能力                           | 详情与文本 | ZIP  | 创建/更新 |
| ----------------------------------- | ---------- | ---- | --------- |
| 无权益游客/普通账号                 | 否         | 否   | 否        |
| 对应 skill 的有效游客凭证或绑定账号 | 是         | 是   | 否        |
| view_all                            | 全部       | 否   | 否        |
| download_all                        | 全部       | 全部 | 否        |
| upload_edit                         | 全部       | 否   | 全部      |

能力可组合。管理员身份本身不隐含内容能力。邮箱登录使用 `hub_session` cookie；游客使用 `X-Guest-Token: <原始token>`，领取/绑定见 [兑换 API](api-redemption.md)。不接受 query 中的 token、客户端 account ID 或对象 key。

写请求必须携带与服务端 `APP_ORIGIN` 完全相同的 `Origin`，且目标 URL origin 也必须匹配。上传要求 `Content-Type: application/json`。所有响应均为 `Cache-Control: no-store`。错误格式是 `{ "error": "stable_code" }`，不包含 SQL、内部对象 key 或凭证。

skill ID 为 1–64 个小写 ASCII 字母、数字、`-`、`_`，首字符必须为字母或数字；创建后不改名。名称为 1–240 UTF-8 bytes，简介为 1–1024 bytes，标签最多 10 个、每项 1–64 bytes，不允许重复；这些文本不能含 control 字符或首尾空白。

## 公开列表

```http
GET /api/skills?limit=50&after=example-skill
```

`limit` 默认 50，范围 1–100；`after` 是上一页的 `nextAfter`，首请求省略。按 skill ID 升序游标分页，未知或重复参数返回 400。分页不是跨请求数据库快照。

```json
{
  "items": [
    {
      "id": "example-skill",
      "name": "示例技能",
      "description": "用于示范的 skill",
      "tags": ["rust"],
      "updatedAt": 1800000000
    }
  ],
  "nextAfter": null
}
```

每个公开条目只有 id、name、description、tags、updatedAt。不返回 revision、文件树、README、对象 key 或下载描述。

## 受限详情与文件清单

```http
GET /api/skills/example-skill
X-Guest-Token: <token>
```

```json
{
  "id": "example-skill",
  "name": "示例技能",
  "description": "用于示范的 skill",
  "tags": ["rust"],
  "updatedAt": 1800000000,
  "revisionId": "<64位小写hex>",
  "files": [
    { "path": "README.md", "byteLen": 7, "isUtf8": true },
    { "path": "assets/icon.bin", "byteLen": 3, "isUtf8": false }
  ],
  "readme": { "path": "README.md", "content": "# Skill" }
}
```

`files` 是按 POSIX 路径排序的平面文件清单，可据此构造文件树。README 选择根目录中大小写不敏感匹配 `README.md` 的首个 UTF-8 文件；没有则为 null。UTF-8 标记仅说明可解码，不是 MIME 或安全内容分类。README 是原始文本，不是服务端渲染 HTML。

授权通过后一次读取当前 revision 快照，公开元数据、清单、README 来自同一 revision。更新不会让本次请求混用两个版本。

## 受限文本读取

```http
GET /api/skills/example-skill/text?path=docs%2Fguide.md
```

必须且只能有一个 `path` 参数；URL 解码一次后应用相同路径规则。

```json
{
  "revisionId": "<64位小写hex>",
  "path": "docs/guide.md",
  "content": "原始 UTF-8 文本"
}
```

单次文本最多 1 MiB。非 UTF-8 文件返回 415 `not_text_file`，不存在的路径返回 404。文本以 JSON 字符串返回，客户端不得直接当作可执行 HTML。

## 最新 ZIP 下载

```http
GET /api/skills/example-skill/download
```

授权成功后读取一次当前 revision 并流式返回已发布 ZIP，不在下载时遍历/重压缩文件。响应包含：

```text
Content-Type: application/zip
Content-Disposition: attachment; filename="example-skill.zip"
Content-Length: <ZIP bytes>
X-Skill-Revision: <revision id>
Cache-Control: no-store
X-Content-Type-Options: nosniff
```

ZIP 使用 Stored 方法，包含与文件清单相同的路径及原始字节，支持二进制。只有私有 Worker 路由，无公开 R2 URL。授权不通过时不会打开 R2 body；对象缺失则报技术错误，不回退旧版本。

## 创建与更新

创建和更新必须有 `upload_edit` 能力。HTTP 在读取请求体、检查其媒体类型或解析 JSON/base64 前，按已认证身份拒绝匿名账号（401）和无编辑权账号（403）；Application 仍独立校验权限。Origin 检查先于此上传准入检查。

创建示例：

```http
POST /api/skills
Origin: http://127.0.0.1:8787
Content-Type: application/json
Cookie: hub_session=<session>
```

```json
{
  "id": "example-skill",
  "name": "示例技能",
  "description": "用于示范的 skill",
  "tags": ["rust"],
  "files": [
    { "path": "README.md", "contentBase64": "IyBTa2lsbA==" },
    { "path": "assets/icon.bin", "contentBase64": "AP+A" }
  ]
}
```

`contentBase64` 使用标准带 padding base64，不接受 data URL、URL-safe 编码或内嵌空白。创建成功为 201：

```json
{
  "id": "example-skill",
  "revisionId": "<64位小写hex>",
  "updatedAt": 1800000000
}
```

同 ID 已存在时返回 409 `revision_conflict`。

更新使用 `PUT /api/skills/example-skill`，body 不含 `id`，必须携带从详情得到的 `expectedRevision`：

```json
{
  "expectedRevision": "<当前revisionId>",
  "name": "示例技能 v2",
  "description": "完整替换后的描述",
  "tags": ["rust", "tools"],
  "files": [{ "path": "README.md", "contentBase64": "IyB2Mg==" }]
}
```

更新成功为 200，响应结构同创建。更新完整替换元数据和文件集合，不是局部文件补丁。不接受未知 body 字段。不存在的更新目标、过期的 expectedRevision、并发输掉 CAS 均返回 409；客户端应重新读详情再决定是否重试，不自动覆盖。

### 文件限制

- 非空集合，最多 200 文件；每文件最多 1 MiB；总原始内容最多 8 MiB。允许空文件。
- 编码后的整个 JSON 请求体最多 12 MiB；按输入流累计限制，不依赖 Content-Length。解码时另检查单文件与总大小。
- JSON 反序列化只构造最多 10 个标签、200 个文件；遇到第 11 个标签或第 201 个文件即返回 400 `invalid_request`，不解析该超限元素或剩余数组。输入流总限额仍在 JSON 解析之前执行。
- 路径是区分大小写的相对 POSIX 路径，最多 1024 UTF-8 bytes。拒绝绝对路径、空段、`.`/`..`、反斜杠、冒号、control 字符（含 NUL）、重复路径，以及 `a`/`a/b` 这种文件与目录冲突。
- 仅接收 path + bytes，不接收 symlink、目录项、文件权限或上传 ZIP。ZIP 内文件固定普通文件权限 0644。

先生成并完整写入不可变 revision 的文件、manifest 和 Stored ZIP，再执行一个 D1 guarded batch 切换当前指针并写 revision 元数据。R2 任一上传失败不会发布；D1 批次失败或 CAS 冲突不影响旧版。失败时对象可能留作未引用 staging，不自动删除，不把提交结果未知当作安全清理依据。

## 错误码

| HTTP | error                                                                              | 含义                                                           |
| ---- | ---------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| 400  | invalid_request                                                                    | JSON/参数/expectedRevision 不合法、未知字段或标签/文件数量超限 |
| 400  | invalid_skill_id / invalid_skill_metadata                                          | 标识或公开元数据不合法                                         |
| 400  | invalid_file_path / duplicate_file_path / file_directory_conflict / empty_file_set | 文件集合不合法                                                 |
| 400  | invalid_base64 / invalid_guest_token                                               | 编码或游客凭证语法不合法                                       |
| 401  | unauthenticated                                                                    | 创建/更新缺少有效账号会话                                      |
| 403  | origin_forbidden                                                                   | 写请求 Origin 不匹配                                           |
| 403  | skill_forbidden                                                                    | 缺少对应查看/下载/上传编辑权限                                 |
| 404  | not_found                                                                          | 路由不匹配，或授权后查无内容/文件                              |
| 405  | method_not_allowed                                                                 | 方法不匹配，附 Allow                                           |
| 409  | revision_conflict                                                                  | ID 已存在、更新目标不存在或 CAS 期望过期                       |
| 413  | payload_too_large                                                                  | 编码体、单文件或原始总大小超限                                 |
| 415  | unsupported_media_type / not_text_file                                             | 上传不是 JSON，或文件不可按 UTF-8 阅读                         |
| 500  | internal_error                                                                     | 存储或其他技术失败；无内部诊断详情                             |

权限拒绝发生在受限元数据读取之前，因此无权请求即使目标不存在也返回 403，不泄露文件树或 README。公开列表不受此隐藏策略影响。
