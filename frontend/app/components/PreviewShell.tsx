'use client'

import { useCallback, useEffect, useState } from 'react'
import type { ReminderPreview, ReminderSettings } from '../../lib/types'
import { getSettings, requestPreview, sendTestEmail } from '../../lib/api'
import { useSession } from '../../lib/session'

export default function PreviewShell() {
  const [settings, setSettings] = useState<ReminderSettings>()
  const [preview, setPreview] = useState<ReminderPreview>()
  const [status, setStatus] = useState('bootstrap')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)
  const [sending, setSending] = useState(false)
  const { user } = useSession()

  const hydrate = useCallback(async () => {
    setLoading(true)
    setError('')
    const settingsRes = await getSettings()
    if (!settingsRes.ok || !settingsRes.data) {
      setError(settingsRes.error || 'no saved settings')
      setLoading(false)
      return
    }
    setSettings(settingsRes.data)
    const previewRes = await requestPreview(settingsRes.data)
    if (!previewRes.ok || !previewRes.data) {
      setError(previewRes.error || 'preview failed')
    } else {
      setPreview(previewRes.data)
      setStatus('preview updated')
    }
    setLoading(false)
  }, [])

  const handleSendTest = useCallback(async () => {
    if (!preview) return
    setSending(true)
    setError('')
    const recipient = settings?.userEmail
    const res = await sendTestEmail(recipient)
    if (res.ok) {
      setStatus('test email sent')
    } else {
      setError(res.error || 'failed to send test email')
    }
    setSending(false)
  }, [preview, settings])

  useEffect(() => {
    hydrate()
  }, [hydrate])

  const isAdmin = user?.role === 'admin' || user?.role === 'dev'

  return (
    <div className="box">
      <h2 className="section-title">Live Email Preview</h2>
      <p>Renders the exact HTML + plaintext copy currently scheduled for delivery.</p>
      <div className="actions">
        <button type="button" onClick={hydrate} disabled={loading}>
          Refresh preview
        </button>
        {isAdmin && (
          <button type="button" onClick={handleSendTest} disabled={loading || sending || !preview} className="button ghost">
            {sending ? 'Sending…' : 'Send test email'}
          </button>
        )}
      </div>
      {loading && <div className="status">Generating preview…</div>}
      {error && <div className="status error">{error}</div>}
      {!loading && !error && preview && (
        <>
          <div className="preview-card">
            <h3>{preview.subject}</h3>
            <p>Language · {preview.generatedLanguage}</p>
            {preview.weatherAdvisory && <p>{preview.weatherAdvisory}</p>}
          </div>
          <div className="email-html" dangerouslySetInnerHTML={{ __html: preview.html }} />
          <pre>{preview.text}</pre>
          {settings && (
            <div className="status">
              Sending daily at {settings.reminderTime} {settings.timezone} to {settings.userEmail}
            </div>
          )}
          <div className="status success">{status}</div>
        </>
      )}
    </div>
  )
}
