import { randomBytes } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const persist = join(root, '.wrangler/state')
const work = join(root, '.local/backend-operator')
const wrangler = join(root, 'node_modules/wrangler/bin/wrangler.js')
const target = ['--env', 'local', '--local', '--persist-to', persist]

function run(args) {
  const result = spawnSync(process.execPath, [wrangler, ...args], {
    cwd: root,
    stdio: 'inherit',
    env: { ...process.env, WRANGLER_SEND_METRICS: 'false' },
  })
  if (result.error) throw result.error
  if (result.status !== 0)
    throw new Error(
      `Local Wrangler operation failed (${result.status ?? result.signal})`,
    )
}

// Matches hub-domain::account::EmailAddress. Do not add quoted or Unicode mailbox syntax here.
function emailAddress(input) {
  if (typeof input !== 'string')
    throw new Error('Specify a verified email address')
  const value = input.trim().replace(/[A-Z]/g, (char) => char.toLowerCase())
  const pieces = value.split('@')
  const [local, domain] = pieces
  if (
    pieces.length !== 2 ||
    value.length > 254 ||
    !/^[\x00-\x7f]+$/.test(value) ||
    !local ||
    local.length > 64 ||
    local.startsWith('.') ||
    local.endsWith('.') ||
    local.includes('..') ||
    !/^[a-z0-9.!#$%&'*+\-/=?^_`{|}~]+$/.test(local) ||
    !domain.includes('.') ||
    !domain
      .split('.')
      .every(
        (part) =>
          part.length > 0 &&
          part.length <= 63 &&
          !part.startsWith('-') &&
          !part.endsWith('-') &&
          /^[a-z0-9-]+$/.test(part),
      )
  ) {
    throw new Error('Invalid ASCII email address')
  }
  return value
}

// Wrangler execute has no bind-parameter CLI. Encode validated input as a SQLite blob literal,
// then CAST to TEXT. The generated SQL contains no user-controlled SQL syntax.
function textLiteral(value) {
  return `CAST(X'${Buffer.from(value, 'utf8').toString('hex')}' AS TEXT)`
}

async function execute(sql) {
  await mkdir(work, { recursive: true })
  const directory = await mkdtemp(join(work, 'operator-'))
  const file = join(directory, 'operation.sql')
  try {
    await writeFile(file, sql, { mode: 0o600 })
    run(['d1', 'execute', 'DB', ...target, '--file', file])
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
}

async function init() {
  const secretFile = join(root, '.dev.vars.local')
  try {
    await writeFile(
      secretFile,
      `AUTH_PEPPER=${randomBytes(32).toString('hex')}\n`,
      { flag: 'wx', mode: 0o600 },
    )
    console.log('Created ignored .dev.vars.local (keep it across restarts).')
  } catch (error) {
    if (error.code !== 'EEXIST') throw error
    const existing = await readFile(secretFile, 'utf8')
    if (!/^AUTH_PEPPER=(?:[a-fA-F0-9]{2}){32,}\r?$/m.test(existing)) {
      throw new Error(
        'Existing .dev.vars.local needs AUTH_PEPPER with >=32 random bytes encoded as hex; file was not changed',
      )
    }
    console.log('Kept existing .dev.vars.local.')
  }
  run(['d1', 'migrations', 'apply', 'DB', ...target])
}

async function main() {
  const [command, ...args] = process.argv.slice(2)
  switch (command) {
    case 'init':
      if (args.length) throw new Error('init takes no arguments')
      await init()
      break
    case 'migrate':
      if (args.length) throw new Error('migrate takes no arguments')
      run(['d1', 'migrations', 'apply', 'DB', ...target])
      break
    case 'outbox': {
      if (args.length !== 1) throw new Error('Usage: outbox <email>')
      const email = textLiteral(emailAddress(args[0]))
      await execute(
        `SELECT recipient, code, expires_at FROM local_mail_outbox WHERE recipient=${email} AND expires_at>unixepoch() ORDER BY created_at DESC;`,
      )
      break
    }
    case 'purge-mail':
      if (args.length) throw new Error('purge-mail takes no arguments')
      await execute(
        'DELETE FROM local_mail_outbox WHERE expires_at<=unixepoch();',
      )
      break
    case 'admin': {
      const [input, ...flags] = args
      const allowed = new Set(['--view-all', '--download-all', '--upload-edit'])
      if (
        flags.some((flag) => !allowed.has(flag)) ||
        new Set(flags).size !== flags.length
      ) {
        throw new Error(
          'Usage: admin <verified-email> [--view-all] [--download-all] [--upload-edit]; remote/custom targets are not supported',
        )
      }
      const email = textLiteral(emailAddress(input))
      const view = Number(flags.includes('--view-all'))
      const download = Number(flags.includes('--download-all'))
      const upload = Number(flags.includes('--upload-edit'))
      // A missing account yields NULL, violating admin_roles.account_id NOT NULL and exiting nonzero.
      // No account is created. Re-runs add explicit grants without silently removing prior grants.
      await execute(`
INSERT INTO admin_roles(account_id, granted_at)
VALUES ((SELECT id FROM accounts WHERE email_key=${email}), unixepoch())
ON CONFLICT(account_id) DO NOTHING;
INSERT INTO account_capabilities(account_id, view_all, download_all, upload_edit)
SELECT id, ${view}, ${download}, ${upload} FROM accounts WHERE email_key=${email}
ON CONFLICT(account_id) DO UPDATE SET
view_all=MAX(account_capabilities.view_all,excluded.view_all),
download_all=MAX(account_capabilities.download_all,excluded.download_all),
upload_edit=MAX(account_capabilities.upload_edit,excluded.upload_edit);
INSERT INTO admin_bootstrap_audit(account_id, initialized_at)
SELECT id, unixepoch() FROM accounts WHERE email_key=${email}
ON CONFLICT(account_id) DO NOTHING;
`)
      console.log(
        'Initialized local administrator; only explicitly requested capabilities were added.',
      )
      break
    }
    default:
      throw new Error(
        'Usage: node scripts/backend-local.mjs init|migrate|outbox <email>|purge-mail|admin <verified-email> [capability flags]',
      )
  }
}

main().catch((error) => {
  console.error(error.message)
  process.exitCode = 1
})
