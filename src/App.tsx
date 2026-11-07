import { BookOpen, Code2, Package } from 'lucide-react'
import { ThemeToggle } from '@/components/theme-toggle'
import { HealthCard } from '@/features/health/health-card'

function App() {
  return (
    <div className="flex min-h-svh flex-col">
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:absolute focus:z-10 focus:bg-background focus:p-3"
      >
        跳到主要内容
      </a>
      <header className="border-b bg-muted/40">
        <div className="mx-auto flex max-w-6xl items-center justify-between gap-4 px-4 py-4 sm:px-6">
          <a
            href="/"
            className="flex items-center gap-3 rounded-sm text-sm focus-visible:outline-2 focus-visible:outline-ring"
          >
            <Package className="size-7" aria-hidden="true" />
            <span>
              <span className="text-muted-foreground">zsen / </span>
              <span className="font-semibold">skills-hub</span>
            </span>
          </a>
          <ThemeToggle />
        </div>
        <nav aria-label="主导航" className="mx-auto max-w-6xl px-4 sm:px-6">
          <a
            href="/"
            aria-current="page"
            className="inline-flex items-center gap-2 border-b-2 border-orange-500 px-3 pt-1 pb-3 text-sm font-medium focus-visible:outline-2 focus-visible:outline-ring"
          >
            <Code2 className="size-4" aria-hidden="true" />
            概览
          </a>
        </nav>
      </header>

      <main
        id="main"
        className="mx-auto grid w-full max-w-6xl flex-1 content-start gap-8 px-4 py-8 sm:px-6 lg:grid-cols-[minmax(0,1fr)_16rem]"
      >
        <section aria-labelledby="overview-title" className="min-w-0 space-y-5">
          <h1 id="overview-title" className="text-xl font-semibold">
            项目概览
          </h1>
          <HealthCard />
        </section>
        <aside aria-labelledby="stack-title" className="space-y-4 lg:pt-1">
          <h2
            id="stack-title"
            className="flex items-center gap-2 text-sm font-semibold"
          >
            <BookOpen className="size-4" aria-hidden="true" />
            技术栈
          </h2>
          <dl className="space-y-4 text-sm">
            <div className="space-y-1">
              <dt className="text-muted-foreground">前端</dt>
              <dd>React · TypeScript · Vite</dd>
              <dd className="text-muted-foreground">
                Tailwind CSS · shadcn/ui
              </dd>
            </div>
            <div className="space-y-1">
              <dt className="text-muted-foreground">后端</dt>
              <dd>Rust · Cloudflare Workers</dd>
            </div>
          </dl>
        </aside>
      </main>
      <footer className="mx-auto w-full max-w-6xl border-t px-4 py-5 text-xs text-muted-foreground sm:px-6">
        skills-hub
      </footer>
    </div>
  )
}

export default App
