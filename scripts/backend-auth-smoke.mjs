// Real local HTTP + D1 concurrency regression. Start `pnpm dev:api` before running.
import assert from 'node:assert/strict'
import { createHash, randomBytes } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const origin = process.env.BACKEND_ORIGIN ?? 'http://127.0.0.1:8787'
const suffix = randomBytes(8).toString('hex')
const email = `auth-${suffix}@example.test`
const literal = (value) =>
  `CAST(X'${Buffer.from(value).toString('hex')}' AS TEXT)`

class SmokeFailure extends Error {}

function checkLocalOrigin(value) {
  let url
  try {
    url = new URL(value)
  } catch {
    throw new SmokeFailure(
      'BACKEND_ORIGIN must be an exact http loopback origin',
    )
  }
  if (
    url.protocol !== 'http:' ||
    !['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname) ||
    url.username !== '' ||
    url.password !== '' ||
    url.origin !== value
  ) {
    throw new SmokeFailure(
      'BACKEND_ORIGIN must be an exact http origin on 127.0.0.1, localhost or [::1] (no credentials, path, query or fragment)',
    )
  }
}

function expectStatus(label, response, expected) {
  if (response.status !== expected) {
    const actual = Number.isInteger(response.status)
      ? response.status
      : 'non-numeric'
    const error = new SmokeFailure(
      `${label}: expected status ${expected}, actual ${actual}`,
    )
    Error.captureStackTrace(error, expectStatus)
    throw error
  }
}

function expectStatuses(label, responses, expected) {
  const counts = new Map()
  for (const response of responses) {
    if (!Number.isInteger(response.status)) {
      throw new SmokeFailure(`${label}: received non-numeric status`)
    }
    counts.set(response.status, (counts.get(response.status) ?? 0) + 1)
  }
  if (
    counts.size !== expected.length ||
    expected.some(([status, count]) => counts.get(status) !== count)
  ) {
    const summary = (entries) =>
      entries.map(([status, count]) => `${status} x ${count}`).join(', ')
    const error = new SmokeFailure(
      `${label}: expected statuses [${summary(expected)}], actual [${summary([...counts])}]`,
    )
    Error.captureStackTrace(error, expectStatuses)
    throw error
  }
}

function sql(command) {
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
      env: { ...process.env, WRANGLER_SEND_METRICS: 'false' },
    },
  )
  assert.equal(
    result.status,
    0,
    'Local D1 command failed (details suppressed to protect outbox codes)',
  )
  const results = JSON.parse(result.stdout)
  assert(results.every((result) => result.success !== false))
  return results.flatMap((result) => result.results ?? [])
}

async function request(path, body, cookie, requestOrigin = origin) {
  const headers = {}
  if (requestOrigin !== null) headers.Origin = requestOrigin
  if (cookie) headers.Cookie = cookie
  if (body !== undefined) headers['Content-Type'] = 'application/json'
  const result = await fetch(`${origin}${path}`, {
    method: body === undefined ? 'GET' : 'POST',
    redirect: 'error',
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  })
  assert.equal(result.headers.get('Cache-Control'), 'no-store')
  return {
    status: result.status,
    cookie: result.headers.get('Set-Cookie'),
    data: await result.json(),
  }
}

const issue = (email) => request('/api/auth/email/challenges', { email })
const verify = (email, challenge, code) =>
  request('/api/auth/email/verify', {
    email,
    challenge_id: challenge.data.challenge_id,
    code,
  })
function mail(email) {
  const rows = sql(
    `SELECT code FROM local_mail_outbox WHERE recipient=${literal(email)} AND expires_at>unixepoch();`,
  )
  assert.equal(
    rows.length,
    1,
    'Expected exactly one active local outbox message',
  )
  return rows[0].code
}
function admin(args, expectedStatus = 0) {
  const result = spawnSync(
    process.execPath,
    [join(root, 'scripts/backend-local.mjs'), 'admin', ...args],
    { cwd: root, encoding: 'utf8' },
  )
  assert.equal(
    result.status,
    expectedStatus,
    'Local admin initialization returned unexpected status',
  )
}

async function main() {
  checkLocalOrigin(origin)
  expectStatus('health', await request('/api/health'), 200)
  expectStatus(
    'issue without Origin',
    await request('/api/auth/email/challenges', { email }, undefined, null),
    403,
  )
  expectStatus(
    'issue with foreign Origin',
    await request(
      '/api/auth/email/challenges',
      { email },
      undefined,
      'https://foreign.invalid',
    ),
    403,
  )
  expectStatus('no HTTP outbox', await request('/api/auth/outbox'), 404)

  const issued = await Promise.all(
    Array.from({ length: 8 }, () => issue(email)),
  )
  expectStatuses('concurrent issue', issued, [
    [202, 1],
    [429, 7],
  ])
  const challenge = issued.find((result) => result.status === 202)
  assert(!('code' in challenge.data))
  const code = mail(email)
  const verified = await Promise.all(
    Array.from({ length: 12 }, () => verify(email, challenge, code)),
  )
  expectStatuses('concurrent verify', verified, [
    [200, 1],
    [401, 11],
  ])
  const login = verified.find((result) => result.status === 200)
  assert.match(login.cookie, /HttpOnly/i)
  assert.match(login.cookie, /SameSite=Lax/i)
  assert.match(login.cookie, /Path=\//i)
  assert(
    !/; Secure/i.test(login.cookie),
    'Explicit local HTTP cookie must be usable',
  )
  const cookie = login.cookie.split(';')[0]
  const me = await request('/api/me', undefined, cookie)
  expectStatus('me after login', me, 200)
  assert.equal(me.data.email, email)
  assert.equal(
    me.data.is_admin,
    false,
    'Registration must never bootstrap an administrator',
  )
  assert.deepEqual(me.data.capabilities, {
    view_all: false,
    download_all: false,
    upload_edit: false,
  })
  assert.equal(
    sql(
      `SELECT COUNT(*) AS n FROM accounts WHERE email_key=${literal(email)};`,
    )[0].n,
    1,
  )
  assert.equal(
    sql(
      `SELECT COUNT(*) AS n FROM sessions WHERE account_id=${literal(me.data.id)};`,
    )[0].n,
    1,
  )
  assert.equal(
    sql(
      `SELECT COUNT(*) AS n FROM local_mail_outbox WHERE recipient=${literal(email)};`,
    )[0].n,
    0,
  )
  const rawToken = cookie.split('=')[1]
  const hash = createHash('sha256').update(rawToken).digest('hex')
  assert.equal(
    sql(
      `SELECT COUNT(*) AS n FROM sessions WHERE token_hash=${literal(hash)};`,
    )[0].n,
    1,
  )
  assert.equal(
    sql(
      `SELECT COUNT(*) AS n FROM sessions WHERE token_hash=${literal(rawToken)};`,
    )[0].n,
    0,
  )

  admin([`absent-${suffix}@example.test`], 1)
  admin([email])
  const roleOnly = await request('/api/me', undefined, cookie)
  expectStatus('me after admin initialization', roleOnly, 200)
  assert.equal(roleOnly.data.is_admin, true)
  assert.deepEqual(roleOnly.data.capabilities, {
    view_all: false,
    download_all: false,
    upload_edit: false,
  })
  admin([email, '--view-all', '--upload-edit'])
  admin([email])
  const granted = await request('/api/me', undefined, cookie)
  expectStatus('me after explicit capability grants', granted, 200)
  assert.deepEqual(granted.data.capabilities, {
    view_all: true,
    download_all: false,
    upload_edit: true,
  })
  assert.equal(
    sql(
      `SELECT COUNT(*) AS n FROM admin_bootstrap_audit WHERE account_id=${literal(me.data.id)};`,
    )[0].n,
    1,
  )

  const wrongEmail = `attempts-${suffix}@example.test`
  const wrongChallenge = await issue(wrongEmail)
  expectStatus('issue for attempts limit', wrongChallenge, 202)
  const actualCode = mail(wrongEmail)
  const wrongCode = actualCode === '000000' ? '111111' : '000000'
  const failures = await Promise.all(
    Array.from({ length: 7 }, () =>
      verify(wrongEmail, wrongChallenge, wrongCode),
    ),
  )
  expectStatuses('concurrent wrong codes', failures, [[401, 7]])
  assert.equal(
    sql(
      `SELECT attempts FROM email_challenges WHERE email_key=${literal(wrongEmail)};`,
    )[0].attempts,
    5,
  )
  expectStatus(
    'correct code after attempts exhausted',
    await verify(wrongEmail, wrongChallenge, actualCode),
    401,
  )
  assert.equal(
    sql(
      `SELECT COUNT(*) AS n FROM accounts WHERE email_key=${literal(wrongEmail)};`,
    )[0].n,
    0,
  )

  const expiredEmail = `expired-${suffix}@example.test`
  const expired = await issue(expiredEmail)
  expectStatus('issue for expiry', expired, 202)
  const expiredCode = mail(expiredEmail)
  sql(
    `UPDATE email_challenges SET created_at=unixepoch()-100, expires_at=unixepoch()-1 WHERE email_key=${literal(expiredEmail)};`,
  )
  expectStatus(
    'expired code',
    await verify(expiredEmail, expired, expiredCode),
    401,
  )
  assert.equal(
    sql(
      `SELECT COUNT(*) AS n FROM accounts WHERE email_key=${literal(expiredEmail)};`,
    )[0].n,
    0,
  )

  const supersededEmail = `resend-${suffix}@example.test`
  const old = await issue(supersededEmail)
  expectStatus('issue before resend', old, 202)
  const oldCode = mail(supersededEmail)
  sql(
    `UPDATE email_issuance SET next_send_at=unixepoch()-1 WHERE email_key=${literal(supersededEmail)};`,
  )
  const current = await issue(supersededEmail)
  expectStatus('resend', current, 202)
  expectStatus(
    'superseded code',
    await verify(supersededEmail, old, oldCode),
    401,
  )
  expectStatus(
    'current code after resend',
    await verify(supersededEmail, current, mail(supersededEmail)),
    200,
  )

  expectStatus('logout', await request('/api/auth/logout', {}, cookie), 200)
  expectStatus(
    'me after logout',
    await request('/api/me', undefined, cookie),
    401,
  )
  expectStatus(
    'repeated logout',
    await request('/api/auth/logout', {}, cookie),
    200,
  )
  console.log(
    `PASS auth: real local D1 concurrency, attempts/expiry/resend guards, hashed session, explicit admin, logout (${email})`,
  )
}

main().catch((error) => {
  // Never print arbitrary error messages, assertion objects or stack headers: they can
  // contain response bodies, verification codes or cookies. Only our fixed labels,
  // numeric diagnostics and this script's source locations are safe to display.
  const message =
    error instanceof SmokeFailure ? error.message : 'auth smoke failed'
  const numbers =
    typeof error?.expected === 'number' && typeof error?.actual === 'number'
      ? ` (expected ${error.expected}, actual ${error.actual})`
      : ''
  const locations =
    typeof error?.stack === 'string'
      ? error.stack
          .split('\n')
          .filter((line) => /^\s+at /.test(line))
          .map((line) => line.match(/backend-auth-smoke\.mjs:\d+:\d+/)?.[0])
          .filter(Boolean)
          .slice(0, 3)
      : []
  console.error(
    `${message}${numbers}${locations.length ? ` at ${locations.join(' <- ')}` : ''}`,
  )
  process.exitCode = 1
})
