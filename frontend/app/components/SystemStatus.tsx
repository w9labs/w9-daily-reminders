'use client'

import useSWR from 'swr'
import { getHealth } from '../../lib/api'
import type { HealthStatus } from '../../lib/types'

const fetcher = async () => {
  const res = await getHealth()
  if (!res.ok) throw new Error(res.error || 'health check failed')
  return res.data!
}

export default function SystemStatus() {
  const { data, error, isLoading } = useSWR<HealthStatus>('health', fetcher, {
    refreshInterval: 30_000,
  })

  return (
    <div className={`status ${error ? 'error' : ''}`}>
      {isLoading && 'pinging orchestrator'}
      {error && error.message}
      {data && (
        <div className="grid-two">
          <div>
            <p>Scheduler · {data.scheduler}</p>
            <p>Google · {data.googleConnected ? 'connected' : 'disconnected'}</p>
          </div>
          <div>
            <p>Next run · {data.nextRun ?? 'unknown'}</p>
            <p>Last send · {data.lastDispatch ?? 'none yet'}</p>
          </div>
        </div>
      )}
    </div>
  )
}
