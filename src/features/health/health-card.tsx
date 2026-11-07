import { CircleCheck, CircleX, LoaderCircle, RefreshCw } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardAction,
  CardContent,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import { useHealth } from './use-health'

export function HealthCard() {
  const { state, refresh } = useHealth()

  return (
    <Card>
      <CardHeader className="border-b">
        <CardTitle>
          <h2>后端健康检查</h2>
        </CardTitle>
        <CardAction>
          <Button
            variant="outline"
            onClick={refresh}
            disabled={state.kind === 'loading'}
          >
            <RefreshCw aria-hidden="true" />
            重新检查
          </Button>
        </CardAction>
      </CardHeader>
      <CardContent>
        <div
          className="flex min-h-40 flex-col justify-center gap-4"
          role="status"
          aria-live="polite"
          aria-atomic="true"
          aria-busy={state.kind === 'loading'}
        >
          {state.kind === 'loading' && (
            <p className="flex items-center gap-2 text-muted-foreground">
              <LoaderCircle
                className="size-4 motion-safe:animate-spin"
                aria-hidden="true"
              />
              正在连接 Rust Worker…
            </p>
          )}
          {state.kind === 'failed' && (
            <div className="space-y-2">
              <p className="flex items-center gap-2 font-medium text-destructive">
                <CircleX className="size-4" aria-hidden="true" />
                连接失败
              </p>
              <p className="text-sm text-muted-foreground">{state.message}</p>
            </div>
          )}
          {state.kind === 'ready' && (
            <>
              <p className="flex items-center gap-2 font-medium text-emerald-700 dark:text-emerald-400">
                <CircleCheck className="size-4" aria-hidden="true" />
                服务正常
                <Badge variant="outline">{state.data.status}</Badge>
              </p>
              <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-2 text-sm">
                <dt className="text-muted-foreground">服务</dt>
                <dd className="font-mono text-xs break-all">
                  {state.data.service}
                </dd>
                <dt className="text-muted-foreground">运行时</dt>
                <dd className="font-mono text-xs break-all">
                  {state.data.runtime}
                </dd>
              </dl>
            </>
          )}
        </div>
        <p className="border-t pt-3 font-mono text-xs text-muted-foreground">
          GET /api/health
        </p>
      </CardContent>
    </Card>
  )
}
