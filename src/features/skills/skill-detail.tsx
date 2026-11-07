import { useState } from 'react'
import ReactMarkdown from 'react-markdown'
import {
  ArrowLeft,
  BookOpen,
  Check,
  Clock3,
  Download,
  FileText,
  Folder,
  KeyRound,
  LockKeyhole,
  ShieldCheck,
} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { AccessDialog } from './access-dialog'
import { getMockFiles, type SkillSummary } from './mock-data'
import { downloadMockZip } from './mock-download'
import { remainingTime, type Access, type MockHub } from './mock-state'
import styles from './skill-detail.module.css'

export function SkillDetail({
  skill,
  access,
  hub,
  onBack,
  onGranted,
}: {
  skill: SkillSummary
  access: Access
  hub: MockHub
  onBack: () => void
  onGranted: (id: string) => void
}) {
  const [path, setPath] = useState('README.md')
  const [preview, setPreview] = useState(true)
  const [downloaded, setDownloaded] = useState(false)
  const [error, setError] = useState('')
  const locked = access.kind === 'locked'
  const guest = access.kind === 'download' && access.source === 'guest'
  const expired = access.kind === 'locked' && access.reason === 'expired'
  const files = locked ? [] : getMockFiles(skill)
  const current = files.find((file) => file.path === path) ?? files[0]
  const folders = [
    ...new Set(
      files
        .filter((file) => file.path.includes('/'))
        .map((file) => file.path.split('/')[0]!),
    ),
  ]

  function handleDownload() {
    if (!hub.canDownloadNow(skill.id)) {
      setError('当前没有下载权限，请检查授权状态。')
      return
    }
    try {
      downloadMockZip(skill, files)
      setDownloaded(true)
      setError('')
    } catch {
      setError('未能生成 ZIP，请重试。')
    }
  }

  function fileButton(filePath: string, label: string) {
    return (
      <button
        type="button"
        aria-current={path === filePath ? 'page' : undefined}
        onClick={() => setPath(filePath)}
        className={`flex min-h-9 w-full items-center gap-2 rounded-md px-2 text-left text-sm outline-none hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring ${path === filePath ? 'bg-accent font-medium' : 'text-muted-foreground'}`}
      >
        <FileText className="size-4 shrink-0" />
        <span className="truncate">{label}</span>
      </button>
    )
  }

  return (
    <div className="space-y-5">
      <Button
        variant="ghost"
        size="sm"
        onClick={onBack}
        className="-ml-2 text-muted-foreground"
      >
        <ArrowLeft />
        返回目录
      </Button>
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="space-y-2">
          <div className="flex flex-wrap items-center gap-3">
            <BookOpen className="size-5 text-muted-foreground" />
            <h1 className="text-2xl font-semibold tracking-tight">
              {skill.name}
            </h1>
            <Badge variant="outline">最新版本</Badge>
          </div>
          <p className="text-sm leading-6 text-muted-foreground">
            {skill.description}
          </p>
        </div>
        <div className="space-y-1">
          <Button
            onClick={handleDownload}
            disabled={access.kind !== 'download'}
            className="bg-emerald-700 text-white hover:bg-emerald-800 disabled:bg-muted disabled:text-muted-foreground dark:bg-emerald-700 dark:hover:bg-emerald-600"
          >
            <Download />
            下载 ZIP
          </Button>
          <p className="text-xs text-muted-foreground">
            {access.kind === 'read-only'
              ? '当前仅有查看权'
              : locked
                ? '需要下载授权'
                : '本地 mock 文件'}
          </p>
        </div>
      </div>
      <div className="flex flex-wrap items-center justify-between gap-3 rounded-lg border bg-muted/30 px-4 py-3 text-sm">
        <div className="flex items-center gap-2">
          {locked ? (
            <LockKeyhole className="size-4 shrink-0 text-muted-foreground" />
          ) : guest ? (
            <Clock3 className="size-4 shrink-0 text-amber-700 dark:text-amber-400" />
          ) : (
            <ShieldCheck className="size-4 shrink-0 text-emerald-700 dark:text-emerald-400" />
          )}
          {locked ? (
            <span>{expired ? '游客凭证已到期' : '需要访问授权'}</span>
          ) : guest ? (
            <span>
              游客凭证 · 剩余{' '}
              <span className="font-mono tabular-nums">
                {remainingTime(access.expiresAt, hub.now)}
              </span>
            </span>
          ) : access.kind === 'read-only' ? (
            <span>查看全部 · 不包含 ZIP 下载权</span>
          ) : (
            <span>账号已授权 · 长期查看与下载</span>
          )}
        </div>
        {locked || guest ? (
          <AccessDialog
            mode={guest || expired ? 'login' : 'redeem'}
            hub={hub}
            skill={skill}
            onGranted={onGranted}
          >
            <Button variant="outline" size="sm">
              <KeyRound />
              {guest || expired ? '登录并永久绑定' : '使用兑换码'}
            </Button>
          </AccessDialog>
        ) : (
          <span className="text-xs text-muted-foreground">
            更新于 {skill.updatedAt}
          </span>
        )}
      </div>
      {locked ? (
        <section className="rounded-lg border border-dashed px-6 py-14 text-center">
          <LockKeyhole className="mx-auto mb-4 size-7 text-muted-foreground" />
          <h2 className="font-semibold">
            {expired ? '临时访问已结束' : '文件尚未解锁'}
          </h2>
          <p className="mx-auto mt-2 max-w-sm text-sm leading-6 text-muted-foreground">
            {expired
              ? '此码不能再次领取游客凭证。登录并绑定原兑换码，即可长期查看与下载。'
              : '输入兑换码后可查看文件、README 并下载 ZIP。游客可领取一次 10 分钟凭证。'}
          </p>
        </section>
      ) : (
        current && (
          <div className="grid items-start gap-5 md:grid-cols-[13rem_minmax(0,1fr)]">
            <nav
              aria-label="文件树"
              className="rounded-lg border p-3 md:sticky md:top-5"
            >
              <div className="mb-3 flex items-center justify-between px-2 text-xs text-muted-foreground">
                <span className="font-semibold text-foreground">文件</span>
                <span>{files.length} 个文件</span>
              </div>
              <ul className="space-y-1">
                {files
                  .filter((file) => !file.path.includes('/'))
                  .map((file) => (
                    <li key={file.path}>{fileButton(file.path, file.path)}</li>
                  ))}
                {folders.map((folder) => (
                  <li key={folder}>
                    <details open>
                      <summary className="flex min-h-9 cursor-pointer items-center gap-2 rounded-md px-2 text-sm hover:bg-muted focus-visible:outline-2">
                        <Folder className="size-4 text-muted-foreground" />
                        {folder}
                      </summary>
                      <ul className="ml-3 border-l pl-2">
                        {files
                          .filter((file) => file.path.startsWith(`${folder}/`))
                          .map((file) => (
                            <li key={file.path}>
                              {fileButton(
                                file.path,
                                file.path.slice(folder.length + 1),
                              )}
                            </li>
                          ))}
                      </ul>
                    </details>
                  </li>
                ))}
              </ul>
            </nav>
            <section
              aria-label="文件内容"
              className="min-w-0 overflow-hidden rounded-lg border"
            >
              <div className="flex min-h-12 flex-wrap items-center justify-between gap-2 border-b bg-muted/40 px-4 py-2">
                <h2 className="flex min-w-0 items-center gap-2 text-sm font-medium">
                  <FileText className="size-4 shrink-0 text-muted-foreground" />
                  <span className="break-all">{current.path}</span>
                </h2>
                {current.path.endsWith('.md') && (
                  <div className="flex gap-1" aria-label="显示方式">
                    <Button
                      size="xs"
                      variant={preview ? 'secondary' : 'ghost'}
                      aria-pressed={preview}
                      onClick={() => setPreview(true)}
                    >
                      预览
                    </Button>
                    <Button
                      size="xs"
                      variant={!preview ? 'secondary' : 'ghost'}
                      aria-pressed={!preview}
                      onClick={() => setPreview(false)}
                    >
                      源码
                    </Button>
                  </div>
                )}
              </div>
              {preview && current.path.endsWith('.md') ? (
                <article className={`${styles.markdown} p-5 sm:p-8`}>
                  <ReactMarkdown
                    components={{ img: ({ alt }) => <span>{alt}</span> }}
                  >
                    {current.content}
                  </ReactMarkdown>
                </article>
              ) : (
                <div className="overflow-x-auto py-4">
                  <pre className="w-max min-w-full text-xs leading-6 sm:text-sm">
                    <code>
                      {current.content.split('\n').map((line, index) => (
                        <span key={index} className="block pr-5">
                          <span
                            aria-hidden="true"
                            className="mr-5 inline-block w-10 text-right text-muted-foreground select-none"
                          >
                            {index + 1}
                          </span>
                          {line || '\u00a0'}
                        </span>
                      ))}
                    </code>
                  </pre>
                </div>
              )}
            </section>
          </div>
        )
      )}
      {!locked && (
        <div className="min-h-5 text-xs" role="status">
          {error ? (
            <span className="text-destructive">{error}</span>
          ) : downloaded ? (
            <span className="flex items-center gap-1 text-muted-foreground">
              <Check className="size-3" />
              已生成本地 ZIP；可再次下载。
            </span>
          ) : null}
        </div>
      )}
    </div>
  )
}
