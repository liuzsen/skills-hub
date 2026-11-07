import { useState, type ReactElement } from 'react'
import { Dialog } from 'radix-ui'
import { ArrowLeft, KeyRound, LockKeyhole, Mail, X } from 'lucide-react'
import { z } from 'zod'
import { Button } from '@/components/ui/button'
import {
  DEMO_EMAIL,
  MOCK_OTP,
  exampleCodes,
  getAccess,
  type MockHub,
} from './mock-state'
import type { SkillSummary } from './mock-data'

type Panel = 'access' | 'login' | 'redeem'

const inputClass =
  'h-10 w-full rounded-md border bg-background px-3 text-sm outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/30'

export function AccessDialog({
  children,
  mode,
  hub,
  skill,
  onGranted,
}: {
  children: ReactElement
  mode: Panel
  hub: MockHub
  skill?: SkillSummary
  onGranted: (id: string) => void
}) {
  const [open, setOpen] = useState(false)
  const [panel, setPanel] = useState(mode)
  const title =
    panel === 'login'
      ? '邮箱验证码登录'
      : panel === 'redeem'
        ? '使用兑换码'
        : '此 Skill 需要授权'

  function closeWithSkill(id: string) {
    setOpen(false)
    onGranted(id)
  }

  return (
    <Dialog.Root
      open={open}
      onOpenChange={(value) => {
        setOpen(value)
        if (value) setPanel(mode)
      }}
    >
      <Dialog.Trigger asChild>{children}</Dialog.Trigger>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 z-40 bg-black/45 backdrop-blur-[2px]" />
        <Dialog.Content className="fixed top-1/2 left-1/2 z-50 max-h-[90svh] w-[calc(100%_-_2rem)] max-w-md -translate-x-1/2 -translate-y-1/2 overflow-y-auto rounded-xl border bg-background p-6 shadow-xl">
          <Dialog.Close asChild>
            <Button
              variant="ghost"
              size="icon"
              className="absolute top-3 right-3"
              aria-label="关闭弹窗"
            >
              <X />
            </Button>
          </Dialog.Close>
          <div className="mb-4 flex size-10 items-center justify-center rounded-lg border bg-muted/40 text-muted-foreground">
            {panel === 'login' ? (
              <Mail className="size-5" />
            ) : panel === 'redeem' ? (
              <KeyRound className="size-5" />
            ) : (
              <LockKeyhole className="size-5" />
            )}
          </div>
          <Dialog.Title className="pr-5 text-lg font-semibold">
            {title}
          </Dialog.Title>
          <Dialog.Description className="mt-2 text-sm leading-6 text-muted-foreground">
            {panel === 'access'
              ? `${skill?.name ?? '内容'}的文件、README 和下载仅对已授权用户开放。`
              : panel === 'login'
                ? '首次验证自动注册，无需设置密码。本演示不会发送邮件。'
                : hub.state.identity.kind === 'guest'
                  ? '游客可领取一次 10 分钟凭证；登录绑定后可长期访问。'
                  : `将兑换码永久绑定到 ${hub.state.identity.email}。`}
          </Dialog.Description>
          {panel === 'access' && (
            <div className="mt-6 space-y-3">
              <Button
                className="h-10 w-full"
                onClick={() => setPanel('redeem')}
              >
                <KeyRound />
                输入兑换码
              </Button>
              <Button
                variant="outline"
                className="h-10 w-full"
                onClick={() => setPanel('login')}
              >
                <Mail />
                登录已有授权账号
              </Button>
              <p className="pt-1 text-xs leading-5 text-muted-foreground">
                登录本身不授予内容权限，仍需绑定兑换码。
              </p>
            </div>
          )}
          {panel === 'login' && (
            <LoginForm
              onSuccess={(email) => {
                const next = hub.login(email)
                if (
                  skill &&
                  getAccess(next, skill.id, Date.now()).kind !== 'locked'
                )
                  closeWithSkill(skill.id)
                else if (mode === 'login' && !skill) setOpen(false)
                else setPanel('redeem')
              }}
            />
          )}
          {panel === 'redeem' && (
            <RedeemForm
              hub={hub}
              skill={skill}
              onSuccess={closeWithSkill}
              onLogin={() => setPanel('login')}
            />
          )}
          {panel !== mode && (
            <Button
              variant="ghost"
              size="sm"
              className="mt-3"
              onClick={() => setPanel(mode)}
            >
              <ArrowLeft />
              返回
            </Button>
          )}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  )
}

function LoginForm({ onSuccess }: { onSuccess: (email: string) => void }) {
  const [email, setEmail] = useState('')
  const [sentTo, setSentTo] = useState<string | null>(null)
  const [otp, setOtp] = useState('')
  const [error, setError] = useState('')

  return (
    <form
      className="mt-5 space-y-4"
      onSubmit={(event) => {
        event.preventDefault()
        const normalized = email.trim().toLowerCase()
        if (!z.email().safeParse(normalized).success) {
          setError('请输入有效的邮箱地址。')
          return
        }
        if (sentTo !== normalized) {
          setSentTo(normalized)
          setError('')
          setOtp('')
          return
        }
        if (otp !== MOCK_OTP) {
          setError(`验证码不正确，本地演示请使用 ${MOCK_OTP}。`)
          return
        }
        onSuccess(normalized)
      }}
    >
      <div className="space-y-2">
        <label htmlFor="login-email" className="text-sm font-medium">
          邮箱地址
        </label>
        <input
          id="login-email"
          type="email"
          autoComplete="email"
          autoFocus
          required
          placeholder={DEMO_EMAIL}
          value={email}
          onChange={(event) => {
            setEmail(event.target.value)
            setError('')
          }}
          className={inputClass}
        />
      </div>
      {sentTo === email.trim().toLowerCase() && (
        <div className="space-y-2">
          <label htmlFor="login-otp" className="text-sm font-medium">
            邮箱验证码
          </label>
          <input
            id="login-otp"
            autoFocus
            inputMode="numeric"
            autoComplete="one-time-code"
            maxLength={6}
            required
            placeholder="6 位验证码"
            value={otp}
            onChange={(event) => {
              setOtp(event.target.value)
              setError('')
            }}
            className={inputClass}
          />
          <p className="text-xs text-muted-foreground">
            模拟验证码已生成，未发送真实邮件。
          </p>
        </div>
      )}
      <p className="rounded-md border border-dashed bg-muted/30 px-3 py-2 text-xs leading-5 text-muted-foreground">
        演示邮箱：{DEMO_EMAIL}
        <br />
        任意有效邮箱均可登录 · 验证码：
        <strong className="font-mono text-foreground">{MOCK_OTP}</strong>
      </p>
      <p role="alert" className="min-h-5 text-xs leading-5 text-destructive">
        {error}
      </p>
      <Button type="submit" className="h-10 w-full">
        {sentTo === email.trim().toLowerCase()
          ? '验证并登录'
          : '获取模拟验证码'}
      </Button>
    </form>
  )
}

function RedeemForm({
  hub,
  skill,
  onSuccess,
  onLogin,
}: {
  hub: MockHub
  skill?: SkillSummary
  onSuccess: (id: string) => void
  onLogin: () => void
}) {
  const [code, setCode] = useState('')
  const [error, setError] = useState('')
  const example =
    exampleCodes.find((item) => item.skillId === skill?.id) ?? exampleCodes[0]!
  const isGuest = hub.state.identity.kind === 'guest'
  return (
    <form
      className="mt-5 space-y-4"
      onSubmit={(event) => {
        event.preventDefault()
        const result = hub.redeem(code)
        if (!result.ok) {
          setError(result.message)
          return
        }
        onSuccess(result.skillId)
      }}
    >
      <div className="space-y-2">
        <label htmlFor="redeem-code" className="text-sm font-medium">
          兑换码
        </label>
        <input
          id="redeem-code"
          autoFocus
          required
          autoComplete="off"
          spellCheck={false}
          placeholder="输入兑换码"
          value={code}
          onChange={(event) => {
            setCode(event.target.value)
            setError('')
          }}
          className={`${inputClass} font-mono uppercase`}
        />
      </div>
      <div className="rounded-md border border-dashed bg-muted/30 px-3 py-2 text-xs leading-6 text-muted-foreground">
        示例码（{example.label}）：
        <button
          type="button"
          className="rounded px-1 font-mono text-foreground underline underline-offset-4 focus-visible:outline-2"
          onClick={() => {
            setCode(example.code)
            setError('')
          }}
        >
          {example.code}
        </button>
        <br />
        他人已绑定：<code>TAKEN-2026</code> · 无效码可任意输入
      </div>
      <p className="text-xs leading-5 text-muted-foreground">
        {isGuest
          ? '同一码只能领取一次。窗口内可重复下载，到期后仍可登录绑定。'
          : '绑定后可长期查看和下载最新版本；不能再供其他账号或游客使用。'}
      </p>
      <p role="alert" className="min-h-10 text-xs leading-5 text-destructive">
        {error}
      </p>
      <Button type="submit" className="h-10 w-full">
        {isGuest ? '领取 10 分钟游客凭证' : '永久绑定并打开'}
      </Button>
      {isGuest && (
        <Button
          type="button"
          variant="outline"
          className="h-10 w-full"
          onClick={onLogin}
        >
          先登录，永久绑定兑换码
        </Button>
      )}
    </form>
  )
}
