import { APP_API_BASE } from './config'
import type { ApiResponse, ReminderPreview, ReminderSettings, HealthStatus } from './types'

async function request<T>(path: string, init?: RequestInit): Promise<ApiResponse<T>> {
  try {
    const headers = new Headers(init?.headers)
    if (!headers.has('content-type')) {
      headers.set('content-type', 'application/json')
    }

    const res = await fetch(`${APP_API_BASE}${path}`, {
      ...init,
      headers,
      cache: 'no-store',
    })

    const contentType = res.headers.get('content-type') || ''
    let body: any = {}
    
    if (contentType.includes('application/json')) {
      try {
        const text = await res.text()
        body = text ? JSON.parse(text) : {}
      } catch (parseError) {
        return { ok: false, error: `Invalid JSON response: ${parseError instanceof Error ? parseError.message : 'parse error'}` }
      }
    } else {
      const text = await res.text()
      return { ok: false, error: text || `HTTP ${res.status}: ${res.statusText}` }
    }

    if (!res.ok) {
      return { ok: false, error: body.error || body.message || `HTTP ${res.status}: ${res.statusText}` }
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

export function sendTestEmail(recipient?: string) {
  return request<{ status: string }>('/api/reminders/send-test', {
    method: 'POST',
    headers: {
      'authorization': `Bearer ${getStoredToken() || ''}`,
    },
    body: JSON.stringify({ recipient }),
  })
}

export function getImageModels() {
  return request<string[]>('/api/system/image-models', { method: 'GET' })
}

function getStoredToken(): string | null {
  if (typeof window === 'undefined') return null
  return localStorage.getItem('w9_token')
}
