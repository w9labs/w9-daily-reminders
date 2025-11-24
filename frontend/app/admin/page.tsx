'use client'

import { FormEvent, useEffect, useMemo, useState } from 'react'
import { useSession } from '../../lib/session'
import { APP_API_BASE } from '../../lib/config'
import { getStoredToken } from '../../lib/auth'

interface SenderSelection {
  address: string
  displayName?: string
}

interface SenderOption {
  id: string
  address: string
  display_name?: string
  kind: 'account' | 'alias'
  is_active: boolean
}

interface ConfigResponse {
  mailApiBase: string
  dailySender?: SenderSelection
  noreplySender?: SenderSelection
  serviceTokenPresent: boolean
}

export default function AdminPage() {
  const { user, loading: sessionLoading } = useSession()
  const [config, setConfig] = useState<ConfigResponse | null>(null)
  const [senders, setSenders] = useState<SenderOption[]>([])
  const [status, setStatus] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)
  const [serviceTokenInput, setServiceTokenInput] = useState('')
  const [mailApiBase, setMailApiBase] = useState('')
  const [selectedDailyId, setSelectedDailyId] = useState<string>('')
  const [selectedNoreplyId, setSelectedNoreplyId] = useState<string>('')

  const isAdmin = user?.role?.toLowerCase() === 'admin'

  const senderMap = useMemo(() => {
    const map = new Map<string, SenderOption>()
    senders.forEach((sender) => {
      map.set(sender.id, sender)
    })
    return map
  }, [senders])

  useEffect(() => {
    if (!isAdmin) return
    async function bootstrap() {
      try {
        const token = getStoredToken()
        if (!token) {
          setError('Missing auth token')
          return
        }
        setError(null)
        const [configResp, sendersResp] = await Promise.all([
          fetch(`${APP_API_BASE}/api/system/config`, {
            headers: { Authorization: `Bearer ${token}` },
          }),
          fetch(`${APP_API_BASE}/api/system/senders`, {
            headers: { Authorization: `Bearer ${token}` },
          }),
        ])
        if (!configResp.ok) {
          const data = await configResp.json().catch(() => ({}))
          throw new Error(data.error || 'Unable to load config')
        }
        if (!sendersResp.ok) {
          const data = await sendersResp.json().catch(() => ({}))
          throw new Error(data.error || 'Unable to load senders')
        }
        const configData = (await configResp.json()).data as ConfigResponse
        const senderData = (await sendersResp.json()).data as SenderOption[]
        setConfig(configData)
        setSenders(senderData)
        setMailApiBase(configData.mailApiBase)
        if (configData.dailySender) {
          const found = senderData.find((s) => s.address === configData.dailySender?.address)
          if (found) {
            setSelectedDailyId(found.id)
          }
        }
        if (configData.noreplySender) {
          const found = senderData.find((s) => s.address === configData.noreplySender?.address)
          if (found) {
            setSelectedNoreplyId(found.id)
          }
        }
      } catch (err: any) {
        setError(err?.message || 'Failed to load admin data')
      }
    }
    bootstrap()
  }, [isAdmin])

  if (sessionLoading) {
    return <div className="box">Loading session…</div>
  }

  if (!isAdmin) {
    return <div className="box">Admin role required.</div>
  }

  async function handleSave(event: FormEvent) {
    event.preventDefault()
    if (!config) return
    const token = getStoredToken()
    if (!token) {
      setError('Missing auth token')
      return
    }
    setSaving(true)
    setError(null)
    setStatus(null)
    try {
      const payload: any = {
        mailApiBase,
      }
      if (serviceTokenInput.trim()) {
        payload.mailServiceToken = serviceTokenInput.trim()
      }
      if (selectedDailyId) {
        const sender = senderMap.get(selectedDailyId)
        if (sender) {
          payload.dailySender = {
            address: sender.address,
            displayName: sender.display_name,
          }
        }
      }
      if (selectedNoreplyId) {
        const sender = senderMap.get(selectedNoreplyId)
        if (sender) {
          payload.noreplySender = {
            address: sender.address,
            displayName: sender.display_name,
          }
        }
      }
      const resp = await fetch(`${APP_API_BASE}/api/system/config`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${token}`,
        },
        body: JSON.stringify(payload),
      })
      const data = await resp.json()
      if (!resp.ok) {
        throw new Error(data.error || 'Failed to update config')
      }
      setConfig(data.data as ConfigResponse)
      setStatus('Configuration saved')
      setServiceTokenInput('')
    } catch (err: any) {
      setError(err?.message || 'Failed to update config')
    } finally {
      setSaving(false)
    }
  }

  async function handleSendTest() {
    const token = getStoredToken()
    if (!token) {
      setError('Missing auth token')
      return
    }
    setStatus(null)
    setError(null)
    try {
      const resp = await fetch(`${APP_API_BASE}/api/reminders/send-test`, {
        method: 'POST',
        headers: {
          Authorization: `Bearer ${token}`,
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({}),
      })
      const data = await resp.json()
      if (!resp.ok) {
        throw new Error(data.error || 'Test send failed')
      }
      setStatus('Test reminder dispatched')
    } catch (err: any) {
      setError(err?.message || 'Test send failed')
    }
  }

  return (
    <div className="box">
      <h2 className="section-title">Admin · Mail Integration</h2>
      <p>Configure which W9 Mail senders this service uses.</p>
      <form className="form" onSubmit={handleSave}>
        <label className="row">
          Mail API base URL
          <input type="url" value={mailApiBase} onChange={(e) => setMailApiBase(e.target.value)} required />
        </label>
        <label className="row">
          Daily reminder sender
          <select value={selectedDailyId} onChange={(e) => setSelectedDailyId(e.target.value)}>
            <option value="">-- choose --</option>
            {senders.map((sender) => (
              <option key={sender.id} value={sender.id}>
                {sender.kind.toUpperCase()} · {sender.address} {sender.display_name ? `(${sender.display_name})` : ''}
              </option>
            ))}
          </select>
        </label>
        <label className="row">
          No-reply sender
          <select value={selectedNoreplyId} onChange={(e) => setSelectedNoreplyId(e.target.value)}>
            <option value="">-- choose --</option>
            {senders.map((sender) => (
              <option key={sender.id} value={sender.id}>
                {sender.kind.toUpperCase()} · {sender.address} {sender.display_name ? `(${sender.display_name})` : ''}
              </option>
            ))}
          </select>
        </label>
        <label className="row">
          Service API token
          <input
            type="password"
            value={serviceTokenInput}
            placeholder={config?.serviceTokenPresent ? 'Token already set · enter to replace' : 'Paste API token from W9 Mail'}
            onChange={(e) => setServiceTokenInput(e.target.value)}
          />
        </label>
        <button type="submit" disabled={saving}>
          {saving ? 'Saving…' : 'Save configuration'}
        </button>
      </form>
      <div className="actions" style={{ marginTop: '1rem' }}>
        <button type="button" className="button ghost" onClick={handleSendTest}>
          Send test reminder
        </button>
      </div>
      {status && <div className="status success">{status}</div>}
      {error && <div className="status error">{error}</div>}
    </div>
  )
}
