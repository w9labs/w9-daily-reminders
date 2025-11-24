import { APP_API_BASE } from './config'
import type { ApiResponse, ReminderPreview, ReminderSettings, HealthStatus } from './types'

async function request<T>(path: string, init?: RequestInit): Promise<ApiResponse<T>> {
  try {
    const res = await fetch(`${APP_API_BASE}${path}`, {
      headers: {
        'content-type': 'application/json',
      },
      ...init,
      cache: 'no-store',
    })

    const body = await res.json().catch(() => ({}))
    if (!res.ok) {
      return { ok: false, error: body.error || 'request failed' }
    }

    return { ok: true, data: body.data as T }
  } catch (error) {
    return { ok: false, error: error instanceof Error ? error.message : 'network error' }
  }
}

export function saveSettings(payload: ReminderSettings) {
  return request<ReminderSettings>('/api/settings', {
    method: 'POST',
    body: JSON.stringify(payload),
  })
}

export function getSettings() {
  return request<ReminderSettings>('/api/settings', { method: 'GET' })
}

export function requestPreview(payload: ReminderSettings) {
  return request<ReminderPreview>('/api/reminders/preview', {
    method: 'POST',
    body: JSON.stringify(payload),
  })
}

export function getHealth() {
  return request<HealthStatus>('/api/system/health', { method: 'GET' })
}

export function startGoogleAuth() {
  return request<{ url: string }>('/api/google/start', { method: 'POST' })
}

export function completeGoogleAuth(code: string) {
  return request<{ connected: boolean }>('/api/google/callback', {
    method: 'POST',
    body: JSON.stringify({ code }),
  })
}
