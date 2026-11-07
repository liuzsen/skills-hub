import { useEffect, useState } from 'react'
import { z } from 'zod'

const healthSchema = z.object({
  status: z.literal('ok'),
  service: z.literal('skills-worker'),
  runtime: z.literal('cloudflare-workers'),
})

type HealthState =
  | { kind: 'loading' }
  | { kind: 'ready'; data: z.infer<typeof healthSchema> }
  | { kind: 'failed'; message: string }

export function useHealth() {
  const [state, setState] = useState<HealthState>({ kind: 'loading' })
  const [request, setRequest] = useState(0)

  useEffect(() => {
    const controller = new AbortController()

    async function checkHealth() {
      setState({ kind: 'loading' })
      try {
        const response = await fetch('/api/health', {
          cache: 'no-store',
          signal: AbortSignal.any([
            controller.signal,
            AbortSignal.timeout(10_000),
          ]),
          headers: { Accept: 'application/json' },
        })
        if (controller.signal.aborted) return
        if (!response.ok) {
          setState({
            kind: 'failed',
            message: `后端返回 HTTP ${response.status}，请检查 Worker 日志。`,
          })
          return
        }
        const result = healthSchema.safeParse(await response.json())
        if (controller.signal.aborted) return
        if (!result.success) {
          setState({
            kind: 'failed',
            message: '健康检查响应格式不正确，请检查 /api/health。',
          })
          return
        }
        setState({ kind: 'ready', data: result.data })
      } catch {
        if (!controller.signal.aborted) {
          setState({
            kind: 'failed',
            message: '无法连接后端，请确认 Worker 已启动后重试。',
          })
        }
      }
    }

    void checkHealth()
    return () => controller.abort()
  }, [request])

  return { state, refresh: () => setRequest((value) => value + 1) }
}
