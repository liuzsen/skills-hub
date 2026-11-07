// Real local HTTP + D1/R2 regression. Start a local Worker before running.
// BACKEND_ORIGIN defaults to http://127.0.0.1:8787; only exact HTTP loopback origins are accepted.
// Uses fresh names on every run; deliberately leaves its own fixtures in local storage.
import assert, { AssertionError } from 'node:assert/strict'
import { createHash, randomBytes } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { unzipSync } from 'fflate'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const origin = process.env.BACKEND_ORIGIN ?? 'http://127.0.0.1:8787'
const prefix = `biz-${randomBytes(8).toString('hex')}`
const primary = `${prefix}-main`
const secondary = `${prefix}-other`
const literal = (value) =>
  `CAST(X'${Buffer.from(value, 'utf8').toString('hex')}' AS TEXT)`
const digest = (value) => createHash('sha256').update(value).digest('hex')
const noCapabilities = {
  view_all: false,
  download_all: false,
  upload_edit: false,
}
let stage = 'local prerequisites'

function validateOrigin(value) {
  const parsed = new URL(value)
  assert.equal(
    parsed.origin,
    value,
    'BACKEND_ORIGIN must be an exact origin without path, credentials, query or fragment',
  )
  assert.equal(parsed.protocol, 'http:', 'BACKEND_ORIGIN must use local HTTP')
  assert(
    ['127.0.0.1', 'localhost', '[::1]'].includes(parsed.hostname),
    'BACKEND_ORIGIN must name an allowed loopback host',
  )
}

function sql(command) {
  // No configurable database, environment, origin, remote flag, or persistence directory.
  // Queries contain generated IDs/digests, never plaintext bearer tokens or redemption codes.
  const result = spawnSync(
    process.execPath,
    [
      join(root, 'node_modules/wrangler/bin/wrangler.js'),
      'd1',
      'execute',
      'DB',
      '--env',
      'local',
      '--local',
      '--persist-to',
      join(root, '.wrangler/state'),
      '--json',
      '--command',
      command,
    ],
    {
      cwd: root,
      encoding: 'utf8',
      timeout: 60_000,
      env: { ...process.env, WRANGLER_SEND_METRICS: 'false' },
    },
  )
  assert.equal(
    result.status,
    0,
    'Local D1 command failed; CLI diagnostics suppressed',
  )
  const results = JSON.parse(result.stdout)
  assert(
    results.every((result) => result.success !== false),
    'Local D1 reported an unsuccessful statement',
  )
  return results.flatMap((result) => result.results ?? [])
}

function bootstrap(email) {
  const result = spawnSync(
    process.execPath,
    [join(root, 'scripts/backend-local.mjs'), 'admin', email],
    {
      cwd: root,
      encoding: 'utf8',
      timeout: 60_000,
      env: { ...process.env, WRANGLER_SEND_METRICS: 'false' },
    },
  )
  assert.equal(
    result.status,
    0,
    'Explicit local administrator bootstrap failed; CLI diagnostics suppressed',
  )
}

async function request(
  path,
  {
    body,
    rawBody,
    cookie,
    guest,
    method = body === undefined && rawBody === undefined ? 'GET' : 'POST',
    requestOrigin = origin,
  } = {},
) {
  assert(
    body === undefined || rawBody === undefined,
    'Request helper must not receive both JSON and raw bodies',
  )
  const headers = {}
  if (requestOrigin !== null) headers.Origin = requestOrigin
  if (cookie) headers.Cookie = cookie
  if (guest) headers['X-Guest-Token'] = guest
  if (body !== undefined || rawBody !== undefined)
    headers['Content-Type'] = 'application/json'
  const result = await fetch(`${origin}${path}`, {
    method,
    headers,
    redirect: 'error',
    signal: AbortSignal.timeout(60_000),
    body:
      rawBody !== undefined
        ? rawBody
        : body === undefined
          ? undefined
          : JSON.stringify(body),
  })
  assert.equal(
    result.headers.get('Cache-Control'),
    'no-store',
    'Every business response must be no-store',
  )
  const bytes = Buffer.from(await result.arrayBuffer())
  const isZip =
    result.headers.get('Content-Type')?.split(';')[0] === 'application/zip'
  return {
    status: result.status,
    headers: result.headers,
    cookie: result.headers.get('Set-Cookie'),
    bytes,
    data: isZip ? undefined : JSON.parse(bytes.toString('utf8')),
  }
}

function status(result, expected) {
  assert.equal(
    result.status,
    expected,
    'Unexpected HTTP status for the current verification stage',
  )
  return result
}

function rejected(result, expectedStatus, code) {
  status(result, expectedStatus)
  assert.deepEqual(
    result.data,
    { error: code },
    'Rejected response must expose only the stable error code',
  )
}

async function login(label) {
  const email = `${prefix}-${label}@example.test`
  const challenge = status(
    await request('/api/auth/email/challenges', { body: { email } }),
    202,
  )
  assert(
    !('code' in challenge.data),
    'Login API must not return a plaintext email code',
  )
  const rows = sql(
    `SELECT code FROM local_mail_outbox WHERE recipient=${literal(email)} AND expires_at>unixepoch();`,
  )
  assert.equal(
    rows.length,
    1,
    'Expected exactly one active local outbox record for the fresh account',
  )
  const verified = status(
    await request('/api/auth/email/verify', {
      body: {
        email,
        challenge_id: challenge.data.challenge_id,
        code: rows[0].code,
      },
    }),
    200,
  )
  assert.equal(
    typeof verified.cookie,
    'string',
    'Successful login must set a session cookie',
  )
  const cookie = verified.cookie.split(';')[0]
  const me = status(await request('/api/me', { cookie }), 200)
  assert.equal(
    me.data.email,
    email,
    'Session must resolve to the freshly registered email',
  )
  assert.equal(
    me.data.is_admin,
    false,
    'Registration must not implicitly grant administrator status',
  )
  assert.deepEqual(
    me.data.capabilities,
    noCapabilities,
    'Registration must not implicitly grant capabilities',
  )
  return { email, cookie, id: me.data.id }
}

async function grant(admin, account, capability) {
  status(
    await request('/api/admin/capabilities', {
      cookie: admin.cookie,
      body: { account_id: account.id, capability },
    }),
    200,
  )
}

function model(label, guidePath = 'docs/guide.txt') {
  return {
    name: `Business regression ${label}`,
    description: 'Public summary for a local regression fixture',
    tags: ['business-smoke'],
    files: [
      ['README.md', Buffer.from(`# Private ${prefix} ${label}\n`, 'utf8')],
      [
        guidePath,
        Buffer.from(`private-text-${prefix}-${label}\n中文正文\n`, 'utf8'),
      ],
      [
        'assets/sample.bin',
        Buffer.concat([
          Buffer.from([0, 255, 128, 192, 0, 10]),
          Buffer.from(label),
        ]),
      ],
    ],
  }
}

function upload(model) {
  return {
    name: model.name,
    description: model.description,
    tags: model.tags,
    files: model.files.map(([path, bytes]) => ({
      path,
      contentBase64: bytes.toString('base64'),
    })),
  }
}

async function publish(id, model, account, expectedRevision) {
  return request(expectedRevision ? `/api/skills/${id}` : '/api/skills', {
    method: expectedRevision ? 'PUT' : 'POST',
    cookie: account.cookie,
    body: {
      ...upload(model),
      ...(expectedRevision ? { expectedRevision } : { id }),
    },
  })
}

async function readRevision(id, identity, expected, revision) {
  const detail = status(await request(`/api/skills/${id}`, identity), 200)
  assert.equal(detail.data.id, id, 'Detail must use the requested skill ID')
  assert.equal(
    detail.data.revisionId,
    revision,
    'Detail must use the expected current revision',
  )
  assert.equal(
    detail.data.name,
    expected.name,
    'Detail metadata must match the same revision as its files',
  )
  assert.equal(
    detail.data.description,
    expected.description,
    'Detail description must match the expected revision',
  )
  assert.deepEqual(
    detail.data.tags,
    expected.tags,
    'Detail tags must match the expected revision',
  )
  assert.deepEqual(
    detail.data.files,
    expected.files
      .map(([path, bytes]) => ({
        path,
        byteLen: bytes.length,
        isUtf8: path !== 'assets/sample.bin',
      }))
      .sort((left, right) =>
        left.path < right.path ? -1 : left.path > right.path ? 1 : 0,
      ),
    'File manifest must contain only the expected revision paths, lengths and encodings',
  )
  assert.deepEqual(
    detail.data.readme,
    { path: 'README.md', content: expected.files[0][1].toString('utf8') },
    'README must match the expected revision',
  )
  for (const [path, bytes] of expected.files) {
    const text = await request(
      `/api/skills/${id}/text?path=${encodeURIComponent(path)}`,
      identity,
    )
    if (path === 'assets/sample.bin') {
      rejected(text, 415, 'not_text_file')
    } else {
      status(text, 200)
      assert.deepEqual(
        text.data,
        { revisionId: revision, path, content: bytes.toString('utf8') },
        'Text content and revision must match the published bytes',
      )
    }
  }
  return detail.data
}

async function zipRevision(id, identity, expected, revision) {
  const result = status(
    await request(`/api/skills/${id}/download`, identity),
    200,
  )
  assert.equal(
    result.headers.get('Content-Type'),
    'application/zip',
    'Download must be served as ZIP',
  )
  assert.equal(
    result.headers.get('Content-Disposition'),
    `attachment; filename="${id}.zip"`,
    'Download filename must be safe',
  )
  assert.equal(
    result.headers.get('X-Content-Type-Options'),
    'nosniff',
    'Download must disable content sniffing',
  )
  assert.equal(
    result.headers.get('X-Skill-Revision'),
    revision,
    'ZIP must use the same expected revision',
  )
  assert.equal(
    Number(result.headers.get('Content-Length')),
    result.bytes.length,
    'ZIP length must match its response header',
  )
  const files = unzipSync(result.bytes)
  assert.deepEqual(
    Object.keys(files).sort(),
    expected.files.map(([path]) => path).sort(),
    'ZIP entries must match the revision manifest exactly',
  )
  for (const [path, bytes] of expected.files) {
    assert(
      Buffer.from(files[path]).equals(bytes),
      'Unzipped bytes must equal uploaded text and binary bytes',
    )
  }
}

async function fullRevision(id, identity, expected, revision) {
  await readRevision(id, identity, expected, revision)
  await zipRevision(id, identity, expected, revision)
}

async function deniedContent(id, identity = {}) {
  for (const path of [
    `/api/skills/${id}`,
    `/api/skills/${id}/text?path=README.md`,
    `/api/skills/${id}/download`,
  ]) {
    rejected(await request(path, identity), 403, 'skill_forbidden')
  }
}

async function publicSummaries(expectedModels) {
  // Our IDs are adjacent after this fresh random prefix, regardless of earlier fixture runs.
  const listing = status(
    await request(`/api/skills?limit=100&after=${prefix}`),
    200,
  )
  assert.deepEqual(
    Object.keys(listing.data).sort(),
    ['items', 'nextAfter'],
    'Catalog envelope must contain only public fields',
  )
  assert(Array.isArray(listing.data.items), 'Catalog items must be an array')
  for (const item of listing.data.items) {
    assert.deepEqual(
      Object.keys(item).sort(),
      ['description', 'id', 'name', 'tags', 'updatedAt'],
      'Public catalog must not leak revision, file tree, README or object descriptors',
    )
  }
  for (const [id, expected] of expectedModels) {
    const item = listing.data.items.find((item) => item.id === id)
    assert(item, 'Freshly published skill must appear in the public catalog')
    assert.equal(
      item.name,
      expected.name,
      'Public metadata must point to the latest published revision',
    )
    assert.equal(
      item.description,
      expected.description,
      'Public description must match the published metadata',
    )
    assert.deepEqual(
      item.tags,
      expected.tags,
      'Public tags must match the published metadata',
    )
    const serialized = JSON.stringify(item)
    for (const [path, bytes] of expected.files) {
      assert(
        !serialized.includes(path),
        'Catalog must not leak private file paths',
      )
      if (path !== 'assets/sample.bin') {
        assert(
          !serialized.includes(bytes.toString('utf8').trim()),
          'Catalog must not leak private text contents',
        )
      }
    }
  }
}

const claim = (code) => request('/api/redemptions/guest', { body: { code } })
const bind = (code, account) =>
  request('/api/redemptions/bind', { cookie: account.cookie, body: { code } })

async function uploadAdmission(nonEditor) {
  // Bounded pressure, not a million-element fixture: exceed the 12 MiB byte cap with one JSON string.
  // Authentication/authorization must take precedence over syntax, schema and byte-limit errors.
  const invalidJson = '{"files":['
  const largeJson = JSON.stringify({ padding: 'A'.repeat(12 * 1024 * 1024) })
  for (const [cookie, expectedStatus, code] of [
    [undefined, 401, 'unauthenticated'],
    [nonEditor.cookie, 403, 'skill_forbidden'],
  ]) {
    for (const [path, method] of [
      ['/api/skills', 'POST'],
      [`/api/skills/${primary}`, 'PUT'],
    ]) {
      for (const rawBody of [invalidJson, largeJson]) {
        rejected(
          await request(path, { cookie, method, rawBody }),
          expectedStatus,
          code,
        )
      }
    }
  }
}

async function invalidPublications(editor, revision) {
  const invalidIds = []
  let index = 0
  async function rejectFiles(files, expectedStatus, code, overrides = {}) {
    const id = `${prefix}-invalid-${index++}`
    invalidIds.push(id)
    rejected(
      await request('/api/skills', {
        cookie: editor.cookie,
        body: { ...upload(model('invalid')), id, files, ...overrides },
      }),
      expectedStatus,
      code,
    )
  }
  const empty = (path) => ({ path, contentBase64: '' })
  for (const path of [
    '../escape.txt',
    '/absolute.txt',
    'docs//bad',
    'docs/../bad',
    'docs/./bad',
    'docs\\bad',
    'C:bad',
    'nul\0bad',
  ]) {
    await rejectFiles([empty(path)], 400, 'invalid_file_path')
  }
  await rejectFiles([empty('same'), empty('same')], 400, 'duplicate_file_path')
  await rejectFiles(
    [empty('a'), empty('a-b'), empty('a/file')],
    400,
    'file_directory_conflict',
  )
  await rejectFiles([], 400, 'empty_file_set')
  await rejectFiles(
    [{ path: 'README.md', contentBase64: 'not base64!' }],
    400,
    'invalid_base64',
  )
  // Count limits are enforced during bounded Serde parsing, not after building an unbounded Vec.
  await rejectFiles([empty('README.md')], 400, 'invalid_request', {
    tags: Array.from({ length: 11 }, (_, index) => `tag-${index}`),
  })
  await rejectFiles(
    Array.from({ length: 201 }, (_, index) => empty(`file-${index}`)),
    400,
    'invalid_request',
  )
  await rejectFiles(
    [
      {
        path: 'large.bin',
        contentBase64: Buffer.alloc(1024 * 1024 + 1).toString('base64'),
      },
    ],
    413,
    'payload_too_large',
  )
  // 8 MiB + 1 decoded byte still encodes below the 12 MiB request cap: exercise the decoded-total guard.
  const megabyte = Buffer.alloc(1024 * 1024).toString('base64')
  await rejectFiles(
    [
      ...Array.from({ length: 8 }, (_, index) => ({
        path: `part-${index}`,
        contentBase64: megabyte,
      })),
      { path: 'one-more-byte', contentBase64: 'AA==' },
    ],
    413,
    'payload_too_large',
  )
  await rejectFiles(
    [
      {
        path: 'encoded-too-large',
        contentBase64: 'A'.repeat(12 * 1024 * 1024),
      },
    ],
    413,
    'payload_too_large',
  )
  assert.equal(
    sql(
      `SELECT COUNT(*) AS n FROM skills WHERE id IN (${invalidIds.map(literal).join(',')});`,
    )[0].n,
    0,
    'Rejected uploads must not create skill records',
  )
  assert.equal(
    sql(
      `SELECT current_revision_id FROM skills WHERE id=${literal(primary)};`,
    )[0].current_revision_id,
    revision,
    'Rejected uploads must not disturb an existing published revision',
  )
}

async function main() {
  assert.equal(
    process.argv.length,
    2,
    'This script accepts no command-line origin, remote or target arguments',
  )
  validateOrigin(origin)
  status(await request('/api/health'), 200)
  stage = 'fresh local accounts and explicit bootstrap'
  const admin = await login('admin')
  const alice = await login('alice')
  const bob = await login('bob')
  const viewer = await login('viewer')
  const downloader = await login('downloader')
  const editor = await login('editor')
  bootstrap(admin.email)
  const roleOnly = status(
    await request('/api/me', { cookie: admin.cookie }),
    200,
  )
  assert.equal(
    roleOnly.data.is_admin,
    true,
    'Bootstrap must explicitly create the administrator role',
  )
  assert.deepEqual(
    roleOnly.data.capabilities,
    noCapabilities,
    'Administrator role alone must not grant content capabilities',
  )
  rejected(
    await publish(primary, model('role-only'), admin),
    403,
    'skill_forbidden',
  )
  await deniedContent(primary, { cookie: admin.cookie })

  stage = 'write Origin and trusted session identity'
  const grantBody = { account_id: admin.id, capability: 'upload_edit' }
  const otherPort = new URL(origin)
  otherPort.port = otherPort.port === '8787' ? '8788' : '8787'
  for (const requestOrigin of [null, `${origin}/`, otherPort.origin]) {
    rejected(
      await request('/api/admin/capabilities', {
        cookie: admin.cookie,
        body: grantBody,
        requestOrigin,
      }),
      403,
      'origin_forbidden',
    )
  }
  rejected(
    await request('/api/admin/capabilities', { body: grantBody }),
    401,
    'unauthenticated',
  )
  rejected(
    await request('/api/admin/capabilities', {
      cookie: alice.cookie,
      body: grantBody,
    }),
    403,
    'admin_required',
  )
  rejected(
    await request('/api/admin/capabilities', {
      cookie: alice.cookie,
      body: { ...grantBody, actor_id: admin.id },
    }),
    400,
    'invalid_request',
  )
  await grant(admin, admin, 'upload_edit')
  assert.deepEqual(
    status(await request('/api/me', { cookie: admin.cookie }), 200).data
      .capabilities,
    { ...noCapabilities, upload_edit: true },
    'Explicit upload grant must not silently grant view/download flags',
  )

  stage = 'real text/binary publication and public projection'
  const v1 = model('v1')
  const other = model('other')
  for (const requestOrigin of [null, 'https://foreign.invalid']) {
    rejected(
      await request('/api/skills', {
        cookie: admin.cookie,
        body: { ...upload(v1), id: primary },
        requestOrigin,
      }),
      403,
      'origin_forbidden',
    )
  }
  const initial = status(await publish(primary, v1, admin), 201).data
  const otherInitial = status(await publish(secondary, other, admin), 201).data
  assert.match(
    initial.revisionId,
    /^[a-f0-9]{64}$/,
    'Published revision ID must be opaque hex',
  )
  await publicSummaries([
    [primary, v1],
    [secondary, other],
  ])
  await deniedContent(primary)
  await deniedContent(primary, { cookie: alice.cookie })
  await readRevision(primary, { cookie: admin.cookie }, v1, initial.revisionId)
  rejected(
    await request(`/api/skills/${primary}/download`, { cookie: admin.cookie }),
    403,
    'skill_forbidden',
  )

  stage = 'administrator code issuance and digest storage'
  const issueBody = { skill_id: primary, count: 3 }
  rejected(
    await request('/api/admin/redemption-codes', {
      cookie: alice.cookie,
      body: issueBody,
    }),
    403,
    'admin_required',
  )
  rejected(
    await request('/api/admin/redemption-codes', {
      cookie: admin.cookie,
      body: issueBody,
      requestOrigin: null,
    }),
    403,
    'origin_forbidden',
  )
  rejected(
    await request('/api/admin/redemption-codes', {
      cookie: admin.cookie,
      body: { ...issueBody, count: 26 },
    }),
    400,
    'invalid_count',
  )
  rejected(
    await request('/api/admin/redemption-codes', {
      cookie: admin.cookie,
      body: { skill_id: `${prefix}-absent`, count: 1 },
    }),
    404,
    'skill_not_found',
  )
  const issued = status(
    await request('/api/admin/redemption-codes', {
      cookie: admin.cookie,
      body: issueBody,
    }),
    201,
  ).data
  assert.equal(
    issued.codes.length,
    3,
    'Issue response must contain exactly the requested number of one-time codes',
  )
  const records = sql(
    `SELECT id, code_hash FROM redemption_codes WHERE created_by=${literal(admin.id)} AND skill_id=${literal(primary)};`,
  )
  assert.equal(
    records.length,
    3,
    'Rejected batches must not create extra code records',
  )
  for (const code of issued.codes) {
    assert.match(
      code.code,
      /^[a-f0-9]{64}$/,
      'Redemption code must be opaque hex',
    )
    assert.equal(
      records.find((row) => row.id === code.id)?.code_hash,
      digest(code.code),
      'Database must contain only the code digest',
    )
  }
  const [used, active, racing] = issued.codes
  rejected(
    await request('/api/redemptions/guest', {
      body: { code: used.code },
      requestOrigin: null,
    }),
    403,
    'origin_forbidden',
  )
  rejected(
    await request('/api/redemptions/bind', {
      cookie: alice.cookie,
      body: { code: used.code },
      requestOrigin: null,
    }),
    403,
    'origin_forbidden',
  )
  rejected(
    await request(`/api/skills/${primary}`, { guest: used.code }),
    403,
    'skill_forbidden',
  )
  rejected(
    await request(`/api/skills/${primary}`, { guest: 'malformed' }),
    400,
    'invalid_guest_token',
  )

  stage = 'concurrent guest claim and real token-scoped R2 reads'
  const claims = await Promise.all(
    Array.from({ length: 8 }, () => claim(used.code)),
  )
  assert.equal(
    claims.filter((result) => result.status === 201).length,
    1,
    'Concurrent guest claims must yield exactly one credential',
  )
  assert.equal(
    claims.filter((result) => result.status === 409).length,
    7,
    'Losing guest claims must be business conflicts',
  )
  for (const result of claims.filter((result) => result.status !== 201))
    rejected(result, 409, 'code_unavailable')
  const guest = claims.find((result) => result.status === 201).data
  assert.match(guest.token, /^[a-f0-9]{64}$/, 'Guest token must be opaque hex')
  assert.equal(
    guest.skill_id,
    primary,
    "Claim receipt must identify the code's skill",
  )
  const claimed = sql(
    `SELECT guest_token_hash, guest_claimed_at, guest_expires_at FROM redemption_codes WHERE id=${literal(used.id)} AND created_by=${literal(admin.id)};`,
  )[0]
  assert.equal(
    claimed.guest_token_hash,
    digest(guest.token),
    'Database must store only the guest token digest',
  )
  assert.equal(
    claimed.guest_expires_at - claimed.guest_claimed_at,
    600,
    'Guest lifetime must be exactly 600 seconds',
  )
  assert.equal(
    guest.expires_at,
    claimed.guest_expires_at,
    'Claim expiry must agree with the database',
  )
  await fullRevision(primary, { guest: guest.token }, v1, initial.revisionId)
  await zipRevision(primary, { guest: guest.token }, v1, initial.revisionId)
  await deniedContent(secondary, { guest: guest.token })
  // Never place a real credential in a URL, even in a negative test (request URLs may be logged).
  rejected(
    await request(
      `/api/skills/${primary}?token=${randomBytes(32).toString('hex')}`,
    ),
    400,
    'invalid_request',
  )

  stage = 'targeted guest expiry without violating migration CHECK'
  const ownCode = `id=${literal(used.id)} AND code_hash=${literal(digest(used.code))} AND skill_id=${literal(primary)} AND created_by=${literal(admin.id)} AND guest_token_hash=${literal(digest(guest.token))}`
  assert.equal(
    sql(`SELECT COUNT(*) AS n FROM redemption_codes WHERE ${ownCode};`)[0].n,
    1,
    "Expiry fixture must target exactly this run's claimed code",
  )
  // Only this freshly issued record is changed. Both timestamps move together to preserve expires=claimed+600.
  sql(
    `UPDATE redemption_codes SET guest_claimed_at=unixepoch()-601, guest_expires_at=unixepoch()-1 WHERE ${ownCode};`,
  )
  const expired = sql(
    `SELECT guest_claimed_at, guest_expires_at, unixepoch() AS server_now FROM redemption_codes WHERE ${ownCode};`,
  )
  assert.equal(
    expired.length,
    1,
    'Expiry fixture must still identify exactly one owned record',
  )
  assert.equal(
    expired[0].guest_expires_at - expired[0].guest_claimed_at,
    600,
    'Expiry mutation must preserve the guest lifetime CHECK',
  )
  assert(
    expired[0].guest_expires_at < expired[0].server_now,
    "Expiry fixture must be in the server's past",
  )
  await deniedContent(primary, { guest: guest.token })
  rejected(await claim(used.code), 409, 'code_unavailable')

  stage = 'used-code account binding race and permanent entitlement'
  const candidates = [alice, bob]
  const bindings = await Promise.all(
    candidates.map((account) => bind(used.code, account)),
  )
  assert.equal(
    bindings.filter((result) => result.status === 200).length,
    1,
    'Two accounts racing to bind must produce exactly one owner',
  )
  assert.equal(
    bindings.filter((result) => result.status === 409).length,
    1,
    'The competing account must lose binding',
  )
  const owner =
    candidates[bindings.findIndex((result) => result.status === 200)]
  const loser = candidates.find((account) => account.id !== owner.id)
  const bound = sql(
    `SELECT bound_account_id FROM redemption_codes WHERE ${ownCode};`,
  )[0]
  assert.equal(
    bound.bound_account_id,
    owner.id,
    'Stored owner must match the successful account',
  )
  const retries = await Promise.all(
    Array.from({ length: 4 }, () => bind(used.code, owner)),
  )
  for (const result of retries) {
    status(result, 200)
    assert.deepEqual(
      result.data,
      { skill_id: primary, bound: true },
      'Same-owner binding retries must be idempotent',
    )
  }
  rejected(await bind(used.code, loser), 409, 'code_unavailable')
  await deniedContent(primary, { cookie: loser.cookie })
  await deniedContent(secondary, { cookie: owner.cookie })
  await fullRevision(primary, { cookie: owner.cookie }, v1, initial.revisionId)
  await zipRevision(primary, { cookie: owner.cookie }, v1, initial.revisionId)

  stage = 'binding preserves existing guest token and blocks new claims'
  const activeGuest = status(await claim(active.code), 201).data
  const beforeBind = sql(
    `SELECT guest_expires_at FROM redemption_codes WHERE id=${literal(active.id)} AND created_by=${literal(admin.id)};`,
  )[0].guest_expires_at
  status(await bind(active.code, owner), 200)
  await fullRevision(
    primary,
    { guest: activeGuest.token },
    v1,
    initial.revisionId,
  )
  assert.equal(
    sql(
      `SELECT guest_expires_at FROM redemption_codes WHERE id=${literal(active.id)} AND created_by=${literal(admin.id)};`,
    )[0].guest_expires_at,
    beforeBind,
    "Binding must not shorten or extend a pre-existing guest token's lifetime",
  )
  rejected(await claim(active.code), 409, 'code_unavailable')
  const [raceGuest, raceBind] = await Promise.all([
    claim(racing.code),
    bind(racing.code, owner),
  ])
  status(raceBind, 200)
  assert(
    [201, 409].includes(raceGuest.status),
    'Claim/bind race must have only the two valid serialized outcomes',
  )
  if (raceGuest.status === 201)
    await fullRevision(
      primary,
      { guest: raceGuest.data.token },
      v1,
      initial.revisionId,
    )
  else rejected(raceGuest, 409, 'code_unavailable')
  rejected(await claim(racing.code), 409, 'code_unavailable')

  stage = 'independent view/download/edit global capabilities'
  for (const [account, capability] of [
    [viewer, 'view_all'],
    [downloader, 'download_all'],
    [editor, 'upload_edit'],
  ]) {
    await grant(admin, account, capability)
    await grant(admin, account, capability)
    const me = status(
      await request('/api/me', { cookie: account.cookie }),
      200,
    ).data
    assert.equal(
      me.is_admin,
      false,
      'Granting content capabilities must not grant administrator status',
    )
    assert.deepEqual(
      me.capabilities,
      { ...noCapabilities, [capability]: true },
      'Only the explicitly granted capability flag may change',
    )
    for (const [id, expected, revision] of [
      [primary, v1, initial.revisionId],
      [secondary, other, otherInitial.revisionId],
    ]) {
      await readRevision(id, { cookie: account.cookie }, expected, revision)
      if (capability === 'download_all')
        await zipRevision(id, { cookie: account.cookie }, expected, revision)
      else
        rejected(
          await request(`/api/skills/${id}/download`, {
            cookie: account.cookie,
          }),
          403,
          'skill_forbidden',
        )
    }
    rejected(
      await request('/api/admin/redemption-codes', {
        cookie: account.cookie,
        body: issueBody,
      }),
      403,
      'admin_required',
    )
  }
  rejected(
    await publish(`${prefix}-viewer-denied`, model('viewer-denied'), viewer),
    403,
    'skill_forbidden',
  )
  rejected(
    await publish(
      `${prefix}-download-denied`,
      model('download-denied'),
      downloader,
    ),
    403,
    'skill_forbidden',
  )
  rejected(
    await publish(
      primary,
      model('viewer-update-denied'),
      viewer,
      initial.revisionId,
    ),
    403,
    'skill_forbidden',
  )
  rejected(
    await publish(
      primary,
      model('bound-update-denied'),
      owner,
      initial.revisionId,
    ),
    403,
    'skill_forbidden',
  )
  const editorCreated = status(
    await publish(`${prefix}-editor`, model('editor-created'), editor),
    201,
  ).data
  assert.match(
    editorCreated.revisionId,
    /^[a-f0-9]{64}$/,
    'Editor must be able to create new skills without administrator role',
  )
  rejected(
    await request(`/api/skills/${prefix}-editor/download`, {
      cookie: editor.cookie,
    }),
    403,
    'skill_forbidden',
  )
  const absentAccount = randomBytes(32).toString('hex')
  rejected(
    await request('/api/admin/capabilities', {
      cookie: admin.cookie,
      body: { account_id: absentAccount, capability: 'view_all' },
    }),
    404,
    'account_not_found',
  )
  assert.equal(
    sql(
      `SELECT COUNT(*) AS n FROM accounts WHERE id=${literal(absentAccount)};`,
    )[0].n,
    0,
    'Grant must not create an unregistered account',
  )

  stage = 'anonymous and non-editor upload admission before JSON parsing'
  await uploadAdmission(viewer)

  stage = 'new revisions automatically follow permanent bindings'
  const v2 = model('v2', 'docs/latest.txt')
  const updated = status(
    await publish(primary, v2, editor, initial.revisionId),
    200,
  ).data
  assert.notEqual(
    updated.revisionId,
    initial.revisionId,
    'Update must publish a distinct immutable revision',
  )
  await fullRevision(primary, { cookie: owner.cookie }, v2, updated.revisionId)
  await fullRevision(
    primary,
    { guest: activeGuest.token },
    v2,
    updated.revisionId,
  )
  rejected(
    await request(`/api/skills/${primary}/text?path=docs%2Fguide.txt`, {
      cookie: owner.cookie,
    }),
    404,
    'not_found',
  )
  await publicSummaries([
    [primary, v2],
    [secondary, other],
  ])

  stage = 'real concurrent publication CAS and coherent latest content'
  const beforeCount = sql(
    `SELECT COUNT(*) AS n FROM skill_revisions WHERE skill_id=${literal(primary)};`,
  )[0].n
  const updates = [
    model('cas-a', 'docs/winner-a.txt'),
    model('cas-b', 'docs/winner-b.txt'),
  ]
  const results = await Promise.all(
    updates.map((candidate) =>
      publish(primary, candidate, editor, updated.revisionId),
    ),
  )
  assert.equal(
    results.filter((result) => result.status === 200).length,
    1,
    'Concurrent updates from one revision must publish exactly one winner',
  )
  assert.equal(
    results.filter((result) => result.status === 409).length,
    1,
    'The losing concurrent update must return revision_conflict',
  )
  rejected(
    results.find((result) => result.status === 409),
    409,
    'revision_conflict',
  )
  const winnerIndex = results.findIndex((result) => result.status === 200)
  const winner = updates[winnerIndex]
  const current = results[winnerIndex].data
  await fullRevision(
    primary,
    { cookie: owner.cookie },
    winner,
    current.revisionId,
  )
  await fullRevision(
    primary,
    { cookie: downloader.cookie },
    winner,
    current.revisionId,
  )
  await publicSummaries([
    [primary, winner],
    [secondary, other],
  ])
  const stored = sql(
    `SELECT s.current_revision_id, r.name FROM skills s JOIN skill_revisions r ON r.id=s.current_revision_id AND r.skill_id=s.id WHERE s.id=${literal(primary)};`,
  )
  assert.equal(
    stored.length,
    1,
    'Current revision pointer must resolve to exactly one committed revision',
  )
  assert.equal(
    stored[0].current_revision_id,
    current.revisionId,
    'D1 pointer must match the winning HTTP publication receipt',
  )
  assert.equal(
    stored[0].name,
    winner.name,
    'D1 metadata must match the winning publication, never the CAS loser',
  )
  assert.equal(
    sql(
      `SELECT COUNT(*) AS n FROM skill_revisions WHERE skill_id=${literal(primary)};`,
    )[0].n,
    beforeCount + 1,
    'Zero-row CAS must not leave a committed losing revision record',
  )
  rejected(
    await publish(primary, model('stale'), editor, updated.revisionId),
    409,
    'revision_conflict',
  )

  stage = 'dangerous paths, duplicate names and upload size limits'
  await invalidPublications(editor, current.revisionId)
  await fullRevision(
    primary,
    { cookie: owner.cookie },
    winner,
    current.revisionId,
  )
  console.log(
    `PASS business: real local D1/R2, claim/bind races, expiry, independent capabilities, latest ZIP bytes and publication CAS (${prefix})`,
  )
}

main().catch((error) => {
  // Every assertion has an explicit non-secret message. Restrict diagnostics to our source line numbers and numeric values.
  // Never dump full actual/expected objects, response bodies, cookies, fetch errors, CLI output or arbitrary stacks.
  const reason =
    error instanceof AssertionError
      ? error.message.split('\n')[0]
      : 'Local operation failed; diagnostic details suppressed'
  const locations = [
    ...String(error?.stack ?? '').matchAll(
      /backend-business-smoke\.mjs:(\d+):(\d+)/g,
    ),
  ]
    .slice(0, 4)
    .map((match) => `backend-business-smoke.mjs:${match[1]}:${match[2]}`)
  const numbers =
    error instanceof AssertionError
      ? [
          ['actual', error.actual],
          ['expected', error.expected],
        ]
          .filter(
            ([, value]) => typeof value === 'number' && Number.isFinite(value),
          )
          .map(([label, value]) => `${label}=${value}`)
      : []
  console.error(`FAIL business during ${stage}: ${reason}`)
  console.error(
    `at ${locations.join(' <- ') || 'unknown source location'}${numbers.length ? `; ${numbers.join(' ')}` : ''}`,
  )
  process.exitCode = 1
})
