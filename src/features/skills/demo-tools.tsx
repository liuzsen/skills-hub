import { ChevronDown, FlaskConical, RotateCcw, TimerOff } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { skills } from './mock-data'
import {
  DEMO_EMAIL,
  MOCK_OTP,
  remainingTime,
  type DemoScene,
  type MockHub,
} from './mock-state'

const scenes: { id: DemoScene; label: string }[] = [
  { id: 'guest', label: '未授权游客' },
  { id: 'guest-active', label: '游客已领取' },
  { id: 'account', label: '账号已授权' },
  { id: 'read-only', label: '查看全部（只读）' },
  { id: 'expired', label: '游客已到期' },
]

export function DemoTools({
  hub,
  onScene,
}: {
  hub: MockHub
  onScene: (scene: DemoScene) => void
}) {
  const hasActiveGuest = hub.state.codes.some(
    (code) => code.guest.kind === 'claimed' && code.guest.expiresAt > hub.now,
  )
  return (
    <details className="group rounded-lg border border-dashed bg-muted/20">
      <summary className="flex cursor-pointer flex-wrap items-center gap-x-3 gap-y-1 rounded-lg px-4 py-3 text-xs text-muted-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring">
        <FlaskConical className="size-4" />
        <span className="font-medium text-foreground">Mock 演示工具</span>
        <span className="flex-1">仅本地 · 刷新重置全部授权与兑换记录</span>
        <ChevronDown className="size-4 transition-transform group-open:rotate-180" />
      </summary>
      <div className="space-y-5 border-t px-4 py-4">
        <div className="space-y-2">
          <h2 className="text-xs font-medium">
            切换场景{' '}
            <span className="font-normal text-muted-foreground">
              · 会重置兑换记录，授权示例为「工程实践」
            </span>
          </h2>
          <div className="flex flex-wrap gap-2">
            {scenes.map((scene) => (
              <Button
                key={scene.id}
                variant="outline"
                size="sm"
                onClick={() => onScene(scene.id)}
              >
                {scene.label}
              </Button>
            ))}
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            disabled={!hasActiveGuest}
            onClick={hub.expireGuest}
          >
            <TimerOff />
            立即使游客凭证到期
          </Button>
          <Button variant="ghost" size="sm" onClick={() => onScene('guest')}>
            <RotateCcw />
            复位全部
          </Button>
          <span className="text-xs text-muted-foreground">
            正常凭证按真实时间倒计时 10 分钟。
          </span>
        </div>
        <div className="overflow-x-auto rounded-md border bg-background">
          <table className="w-full text-left text-xs">
            <caption className="sr-only">可用演示兑换码与当前状态</caption>
            <thead className="border-b bg-muted/40 text-muted-foreground">
              <tr>
                <th className="px-3 py-2 font-medium">示例码</th>
                <th className="px-3 py-2 font-medium">Skill</th>
                <th className="px-3 py-2 font-medium">状态</th>
              </tr>
            </thead>
            <tbody className="divide-y">
              {hub.state.codes.map((code) => (
                <tr key={code.code}>
                  <td className="px-3 py-2 font-mono whitespace-nowrap select-all">
                    {code.code}
                  </td>
                  <td className="px-3 py-2 whitespace-nowrap">
                    {skills.find((skill) => skill.id === code.skillId)?.name}
                  </td>
                  <td className="px-3 py-2 whitespace-nowrap text-muted-foreground">
                    {code.owner
                      ? hub.state.identity.kind === 'account' &&
                        code.owner === hub.state.identity.email
                        ? '已绑定当前账号'
                        : '已绑定其他账号'
                      : code.guest.kind === 'unused'
                        ? '未领取 / 未绑定'
                        : code.guest.expiresAt > hub.now
                          ? `游客剩余 ${remainingTime(code.guest.expiresAt, hub.now)} · 仍可绑定`
                          : '游客已到期 · 仍可绑定'}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <p className="text-xs leading-5 text-muted-foreground">
          验证码：
          <code className="font-semibold text-foreground">{MOCK_OTP}</code>
          ，任意有效邮箱可模拟登录（如 {DEMO_EMAIL}
          ）。已领取的码不能再次领取，但可永久绑定；他人已绑定的码会被拒绝。所有文件均为演示素材。
        </p>
      </div>
    </details>
  )
}
