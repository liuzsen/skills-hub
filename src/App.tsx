import { useState } from 'react'
import {
  BookOpen,
  KeyRound,
  LogOut,
  Mail,
  Package,
  UserRound,
} from 'lucide-react'
import { ThemeToggle } from '@/components/theme-toggle'
import { Button } from '@/components/ui/button'
import { AccessDialog } from '@/features/skills/access-dialog'
import { DemoTools } from '@/features/skills/demo-tools'
import { skills } from '@/features/skills/mock-data'
import {
  getAccess,
  useMockHub,
  type DemoScene,
} from '@/features/skills/mock-state'
import { SkillCatalog } from '@/features/skills/skill-catalog'
import { SkillDetail } from '@/features/skills/skill-detail'

function App() {
  const hub = useMockHub()
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [sceneRevision, setSceneRevision] = useState(0)
  const skill = skills.find((item) => item.id === selectedId)
  const access = skill ? getAccess(hub.state, skill.id, hub.now) : null
  const { identity } = hub.state

  function switchScene(scene: DemoScene) {
    hub.switchScene(scene)
    setSelectedId(null)
    setSceneRevision((value) => value + 1)
  }

  return (
    <div className="flex min-h-svh flex-col">
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:absolute focus:z-50 focus:bg-background focus:p-3"
      >
        跳到主要内容
      </a>
      <header className="border-b bg-muted/35">
        <div className="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-4 px-4 py-5 sm:px-6">
          <button
            type="button"
            onClick={() => setSelectedId(null)}
            className="flex items-center gap-3 rounded-sm text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
            aria-label="Skills Hub，返回目录"
          >
            <span className="flex size-9 items-center justify-center rounded-lg bg-foreground text-background">
              <Package className="size-5" />
            </span>
            <span>
              <span className="text-muted-foreground">zsen / </span>
              <span className="font-semibold">skills-hub</span>
            </span>
          </button>
          <div className="flex items-center gap-2">
            <AccessDialog mode="redeem" hub={hub} onGranted={setSelectedId}>
              <Button variant="outline">
                <KeyRound />
                <span>兑换码</span>
              </Button>
            </AccessDialog>
            {identity.kind === 'guest' ? (
              <AccessDialog mode="login" hub={hub} onGranted={setSelectedId}>
                <Button variant="outline">
                  <Mail />
                  登录
                </Button>
              </AccessDialog>
            ) : (
              <Button
                variant="ghost"
                size="icon"
                title="退出登录（保留兑换记录）"
                aria-label="退出登录，保留兑换记录"
                onClick={hub.logout}
              >
                <LogOut />
              </Button>
            )}
            <ThemeToggle />
          </div>
        </div>
        <nav
          aria-label="主导航"
          className="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-x-4 gap-y-2 px-4 sm:px-6"
        >
          <button
            type="button"
            onClick={() => setSelectedId(null)}
            aria-current={!skill ? 'page' : undefined}
            className="inline-flex items-center gap-2 border-b-2 border-orange-500 px-3 pt-1 pb-3 text-sm font-medium outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            <BookOpen className="size-4" />
            Skills
            <span className="rounded-full bg-muted px-1.5 text-xs text-muted-foreground">
              {skills.length}
            </span>
          </button>
          <span className="mb-3 flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
            <UserRound className="size-3.5 shrink-0" />
            <span className="max-w-60 truncate">
              {identity.kind === 'guest' ? '游客' : identity.email}
            </span>
            {identity.kind === 'account' &&
              hub.state.accountGrants[identity.email] === 'read-all' && (
                <span>· 只读</span>
              )}
          </span>
        </nav>
      </header>
      <main
        id="main"
        className="mx-auto w-full max-w-6xl flex-1 space-y-10 px-4 py-7 sm:px-6 sm:py-9"
      >
        {!skill || !access ? (
          <SkillCatalog key={sceneRevision} hub={hub} onOpen={setSelectedId} />
        ) : (
          <SkillDetail
            key={skill.id}
            skill={skill}
            access={access}
            hub={hub}
            onBack={() => setSelectedId(null)}
            onGranted={setSelectedId}
          />
        )}
        <DemoTools hub={hub} onScene={switchScene} />
      </main>
      <footer className="mx-auto flex w-full max-w-6xl flex-wrap items-center justify-between gap-2 border-t px-4 py-5 text-xs text-muted-foreground sm:px-6">
        <span>zsen / skills-hub</span>
        <span>本地交互原型 · 不连接真实服务</span>
      </footer>
    </div>
  )
}

export default App
