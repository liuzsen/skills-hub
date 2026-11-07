import { useEffect, useState } from 'react'

export const MOCK_OTP = '246810'
export const DEMO_EMAIL = 'demo@example.com'
export const GUEST_WINDOW_MS = 10 * 60 * 1000

export type Identity = { kind: 'guest' } | { kind: 'account'; email: string }

type AccountGrant = 'read-all' | 'download-all'

type GuestClaim = { kind: 'unused' } | { kind: 'claimed'; expiresAt: number }
type RedemptionCode = {
  code: string
  skillId: string
  owner: string | null
  guest: GuestClaim
}
export type MockState = {
  identity: Identity
  codes: RedemptionCode[]
  accountGrants: Record<string, AccountGrant>
}
export type DemoScene =
  'guest' | 'guest-active' | 'account' | 'read-only' | 'expired'
export type Access =
  | { kind: 'locked'; reason: 'expired' | 'unauthorized' }
  | { kind: 'read-only' }
  | { kind: 'download'; source: 'account' }
  | { kind: 'download'; source: 'guest'; expiresAt: number }

export const exampleCodes = [
  { code: 'ENGINE-2026', skillId: 'engineering', label: '工程实践' },
  { code: 'WRITE-2026', skillId: 'writing', label: '技术文档' },
  { code: 'DESIGN-2026', skillId: 'interface', label: '界面设计' },
  { code: 'RESEARCH-2026', skillId: 'research', label: '资料研究' },
  { code: 'DEBUG-2026', skillId: 'debugging', label: '问题定位' },
  { code: 'PLAN-2026', skillId: 'planning', label: '需求澄清' },
]

export function createMockState(
  scene: DemoScene = 'guest',
  now = Date.now(),
): MockState {
  const codes: RedemptionCode[] = exampleCodes.map(({ code, skillId }) => ({
    code,
    skillId,
    owner: null,
    guest: { kind: 'unused' },
  }))
  codes.push({
    code: 'TAKEN-2026',
    skillId: 'engineering',
    owner: 'other@example.com',
    guest: { kind: 'unused' },
  })
  if (scene === 'guest-active' || scene === 'expired') {
    codes[0]!.guest = {
      kind: 'claimed',
      expiresAt: scene === 'expired' ? now - 1 : now + GUEST_WINDOW_MS,
    }
  }
  if (scene === 'account') codes[0]!.owner = DEMO_EMAIL
  const identity: Identity =
    scene === 'account' || scene === 'read-only'
      ? { kind: 'account', email: DEMO_EMAIL }
      : { kind: 'guest' }
  const accountGrants: Record<string, AccountGrant> =
    scene === 'read-only' ? { [DEMO_EMAIL]: 'read-all' } : {}
  return { identity, codes, accountGrants }
}

export function getAccess(
  state: MockState,
  skillId: string,
  now: number,
): Access {
  const { identity } = state
  if (identity.kind === 'account') {
    const grant = state.accountGrants[identity.email]
    if (
      grant === 'download-all' ||
      state.codes.some(
        (code) => code.skillId === skillId && code.owner === identity.email,
      )
    ) {
      return { kind: 'download', source: 'account' }
    }
    if (grant === 'read-all') return { kind: 'read-only' }
    return { kind: 'locked', reason: 'unauthorized' }
  }
  const claims = state.codes.filter(
    (code) =>
      code.skillId === skillId &&
      code.owner === null &&
      code.guest.kind === 'claimed',
  )
  for (const code of claims) {
    if (code.guest.kind === 'claimed' && code.guest.expiresAt > now) {
      return {
        kind: 'download',
        source: 'guest',
        expiresAt: code.guest.expiresAt,
      }
    }
  }
  return { kind: 'locked', reason: claims.length ? 'expired' : 'unauthorized' }
}

type RedeemResult =
  | { ok: false; message: string }
  | { ok: true; state: MockState; skillId: string }

export function redeemCode(
  state: MockState,
  input: string,
  now: number,
): RedeemResult {
  const code = state.codes.find(
    (item) => item.code === input.trim().toUpperCase(),
  )
  if (!code) return { ok: false, message: '未找到这个兑换码，请检查后重试。' }
  const { identity } = state
  if (code.owner !== null) {
    if (identity.kind === 'account' && code.owner === identity.email) {
      return { ok: true, state, skillId: code.skillId }
    }
    return { ok: false, message: '此兑换码已绑定其他账号，无法领取或绑定。' }
  }
  if (identity.kind === 'guest' && code.guest.kind === 'claimed') {
    return {
      ok: false,
      message:
        code.guest.expiresAt > now
          ? '此码已领取游客凭证，请直接打开对应 skill。也可以登录后永久绑定。'
          : '此码的游客凭证已过期，不能再次领取。登录后仍可永久绑定。',
    }
  }
  const updated: RedemptionCode =
    identity.kind === 'account'
      ? { ...code, owner: identity.email }
      : {
          ...code,
          guest: { kind: 'claimed', expiresAt: now + GUEST_WINDOW_MS },
        }
  return {
    ok: true,
    skillId: code.skillId,
    state: {
      ...state,
      codes: state.codes.map((item) =>
        item.code === code.code ? updated : item,
      ),
    },
  }
}

export function useMockHub() {
  const [state, setState] = useState(() => createMockState())
  const [now, setNow] = useState(Date.now)
  useEffect(() => {
    const refreshClock = () => setNow(Date.now())
    const timer = window.setInterval(refreshClock, 1000)
    window.addEventListener('focus', refreshClock)
    document.addEventListener('visibilitychange', refreshClock)
    return () => {
      window.clearInterval(timer)
      window.removeEventListener('focus', refreshClock)
      document.removeEventListener('visibilitychange', refreshClock)
    }
  }, [])

  function redeem(input: string) {
    const result = redeemCode(state, input, Date.now())
    if (result.ok) setState(result.state)
    setNow(Date.now())
    return result
  }

  function canDownloadNow(skillId: string) {
    return getAccess(state, skillId, Date.now()).kind === 'download'
  }

  function login(email: string) {
    const next: MockState = { ...state, identity: { kind: 'account', email } }
    setState(next)
    return next
  }

  function logout() {
    setState((current) => ({ ...current, identity: { kind: 'guest' } }))
  }

  function switchScene(scene: DemoScene) {
    const time = Date.now()
    setState(createMockState(scene, time))
    setNow(time)
  }

  function expireGuest() {
    const time = Date.now()
    setState((current) => ({
      ...current,
      codes: current.codes.map((code) => ({
        ...code,
        guest:
          code.guest.kind === 'claimed'
            ? { kind: 'claimed', expiresAt: time - 1 }
            : code.guest,
      })),
    }))
    setNow(time)
  }

  return {
    state,
    now,
    redeem,
    canDownloadNow,
    login,
    logout,
    switchScene,
    expireGuest,
  }
}

export type MockHub = ReturnType<typeof useMockHub>

export function remainingTime(expiresAt: number, now: number) {
  const seconds = Math.max(0, Math.ceil((expiresAt - now) / 1000))
  return `${Math.floor(seconds / 60)
    .toString()
    .padStart(2, '0')}:${(seconds % 60).toString().padStart(2, '0')}`
}
