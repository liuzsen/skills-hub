import { useState } from 'react'
import {
  ArrowRight,
  BookOpen,
  Check,
  LockKeyhole,
  Search,
  SlidersHorizontal,
  X,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { AccessDialog } from './access-dialog'
import { skills } from './mock-data'
import { getAccess, type MockHub } from './mock-state'

export function SkillCatalog({
  hub,
  onOpen,
}: {
  hub: MockHub
  onOpen: (id: string) => void
}) {
  const [query, setQuery] = useState('')
  const [tag, setTag] = useState('全部')
  const [permission, setPermission] = useState('all')
  const [sort, setSort] = useState('updated')
  const tags = [...new Set(skills.flatMap((skill) => skill.tags))]
  const filtered = skills
    .filter((skill) => {
      const matchesQuery =
        `${skill.name} ${skill.description} ${skill.tags.join(' ')} ${skill.id}`
          .toLowerCase()
          .includes(query.trim().toLowerCase())
      const unlocked = getAccess(hub.state, skill.id, hub.now).kind !== 'locked'
      return (
        matchesQuery &&
        (tag === '全部' || skill.tags.includes(tag)) &&
        (permission === 'all' ||
          (permission === 'available' ? unlocked : !unlocked))
      )
    })
    .sort((a, b) =>
      sort === 'name'
        ? a.name.localeCompare(b.name, 'zh-CN')
        : b.updatedAt.localeCompare(a.updatedAt),
    )

  function resetFilters() {
    setQuery('')
    setTag('全部')
    setPermission('all')
    setSort('updated')
  }

  return (
    <div className="space-y-7">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Skills 目录</h1>
          <p className="mt-2 text-sm text-muted-foreground">
            {skills.length} 个工作技能 · 只提供最新版本
          </p>
        </div>
        <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <LockKeyhole className="size-3.5" />
          文件与下载需授权
        </span>
      </div>
      <div className="grid gap-6 lg:grid-cols-[11rem_minmax(0,1fr)] lg:gap-9">
        <aside aria-label="分类筛选" className="space-y-3">
          <h2 className="flex items-center gap-2 text-xs font-semibold text-muted-foreground">
            <SlidersHorizontal className="size-3.5" />
            按标签浏览
          </h2>
          <div className="flex flex-wrap gap-1 lg:flex-col">
            {['全部', ...tags].map((item) => (
              <button
                key={item}
                type="button"
                aria-pressed={tag === item}
                onClick={() => setTag(item)}
                className={`flex items-center justify-between gap-5 rounded-md px-3 py-2 text-left text-sm transition-colors outline-none hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring ${tag === item ? 'bg-muted font-medium' : 'text-muted-foreground'}`}
              >
                <span>{item}</span>
                <span className="text-xs text-muted-foreground tabular-nums">
                  {item === '全部'
                    ? skills.length
                    : skills.filter((skill) => skill.tags.includes(item))
                        .length}
                </span>
              </button>
            ))}
          </div>
          <p className="hidden border-t pt-4 text-xs leading-6 text-muted-foreground lg:block">
            兑换码解锁对应 Skill。
            <br />
            登录绑定，长期获取更新。
          </p>
        </aside>
        <section aria-label="Skills 列表" className="min-w-0 space-y-4">
          <div className="flex flex-wrap gap-2">
            <div className="relative min-w-48 flex-1">
              <Search className="pointer-events-none absolute top-3 left-3 size-4 text-muted-foreground" />
              <input
                type="search"
                aria-label="搜索 Skills"
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="搜索名称、简介或标签…"
                className="h-10 w-full rounded-md border bg-background pr-8 pl-9 text-sm outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/30"
              />
              {query && (
                <button
                  type="button"
                  aria-label="清空搜索"
                  className="absolute top-2 right-2 rounded p-1 text-muted-foreground hover:bg-muted focus-visible:outline-2"
                  onClick={() => setQuery('')}
                >
                  <X className="size-4" />
                </button>
              )}
            </div>
            <select
              aria-label="按访问权限筛选"
              value={permission}
              onChange={(event) => setPermission(event.target.value)}
              className="h-10 rounded-md border bg-background px-3 text-sm focus-visible:outline-2 focus-visible:outline-ring"
            >
              <option value="all">全部权限</option>
              <option value="available">我可查看</option>
              <option value="locked">待授权</option>
            </select>
            <select
              aria-label="排序"
              value={sort}
              onChange={(event) => setSort(event.target.value)}
              className="h-10 rounded-md border bg-background px-3 text-sm focus-visible:outline-2 focus-visible:outline-ring"
            >
              <option value="updated">最近更新</option>
              <option value="name">名称排序</option>
            </select>
          </div>
          <p role="status" className="text-xs text-muted-foreground">
            {filtered.length} 个结果{tag !== '全部' ? ` · ${tag}` : ''}
          </p>
          <div className="grid gap-4 sm:grid-cols-2">
            {filtered.map((skill) => {
              const access = getAccess(hub.state, skill.id, hub.now)
              const locked = access.kind === 'locked'
              const entry = (
                <Button variant="ghost" size="sm" className="-mr-2 text-xs">
                  {locked ? '获取访问权' : '打开文件'}
                  <ArrowRight className="size-3.5" />
                </Button>
              )
              return (
                <article
                  key={skill.id}
                  className="flex min-w-0 flex-col rounded-lg border bg-card p-5 transition-colors hover:border-foreground/25"
                >
                  <div className="mb-3 flex items-start justify-between gap-2">
                    <BookOpen className="size-5 text-muted-foreground" />
                    <span
                      className={`flex items-center gap-1 text-xs ${locked ? 'text-muted-foreground' : 'text-emerald-700 dark:text-emerald-400'}`}
                    >
                      {locked ? (
                        <LockKeyhole className="size-3" />
                      ) : (
                        <Check className="size-3" />
                      )}
                      {locked
                        ? access.reason === 'expired'
                          ? '游客已到期'
                          : '待授权'
                        : access.kind === 'read-only'
                          ? '仅查看'
                          : access.source === 'guest'
                            ? '游客限时'
                            : '已授权'}
                    </span>
                  </div>
                  <h2 className="text-base font-semibold">
                    {locked ? (
                      <AccessDialog
                        mode="access"
                        hub={hub}
                        skill={skill}
                        onGranted={onOpen}
                      >
                        <button
                          type="button"
                          className="rounded text-blue-700 underline-offset-4 outline-none hover:underline focus-visible:ring-2 focus-visible:ring-ring dark:text-blue-400"
                        >
                          {skill.name}
                        </button>
                      </AccessDialog>
                    ) : (
                      <button
                        type="button"
                        onClick={() => onOpen(skill.id)}
                        className="rounded text-blue-700 underline-offset-4 outline-none hover:underline focus-visible:ring-2 focus-visible:ring-ring dark:text-blue-400"
                      >
                        {skill.name}
                      </button>
                    )}
                  </h2>
                  <p className="mt-2 flex-1 text-sm leading-6 text-muted-foreground">
                    {skill.description}
                  </p>
                  <div className="mt-4 flex flex-wrap gap-1.5">
                    {skill.tags.map((item) => (
                      <Badge
                        key={item}
                        variant="secondary"
                        className="bg-blue-50 text-blue-800 dark:bg-blue-950/60 dark:text-blue-300"
                      >
                        {item}
                      </Badge>
                    ))}
                  </div>
                  <div className="mt-5 flex flex-wrap items-center justify-between gap-1 border-t pt-3">
                    <span className="text-xs text-muted-foreground">
                      更新于 {skill.updatedAt}
                    </span>
                    {locked ? (
                      <AccessDialog
                        mode="access"
                        hub={hub}
                        skill={skill}
                        onGranted={onOpen}
                      >
                        {entry}
                      </AccessDialog>
                    ) : (
                      <Button
                        variant="ghost"
                        size="sm"
                        className="-mr-2 text-xs"
                        onClick={() => onOpen(skill.id)}
                      >
                        打开文件
                        <ArrowRight className="size-3.5" />
                      </Button>
                    )}
                  </div>
                </article>
              )
            })}
          </div>
          {filtered.length === 0 && (
            <div className="rounded-lg border border-dashed py-16 text-center">
              <Search className="mx-auto mb-3 size-6 text-muted-foreground" />
              <h2 className="text-sm font-medium">没有匹配的 Skills</h2>
              <p className="mt-2 text-xs text-muted-foreground">
                试试其他关键词，或清除当前筛选。
              </p>
              <Button variant="outline" className="mt-5" onClick={resetFilters}>
                清除筛选
              </Button>
            </div>
          )}
        </section>
      </div>
    </div>
  )
}
