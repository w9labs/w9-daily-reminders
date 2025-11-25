'use client'

import { useCallback, useEffect, useState } from 'react'
import type { ReminderPreview, ReminderSettings } from '../../lib/types'
import { getSettings, requestPreview, sendTestEmail } from '../../lib/api'
import { useSession } from '../../lib/session'

export default function PreviewShell() {
  const [settings, setSettings] = useState<ReminderSettings>()
  const [preview, setPreview] = useState<ReminderPreview>()
  const [status, setStatus] = useState('Loading settings…')
  const [error, setError] = useState('')
  const [settingsLoading, setSettingsLoading] = useState(true)
  const [previewLoading, setPreviewLoading] = useState(false)
  const [sending, setSending] = useState(false)
  const [hasPreview, setHasPreview] = useState(false)
  const { user } = useSession()

  const fetchLatestSettings = useCallback(async () => {
    const settingsRes = await getSettings()
    if (!settingsRes.ok || !settingsRes.data) {
      throw new Error(settingsRes.error || 'no saved settings')
    }
    setSettings(settingsRes.data)
    return settingsRes.data
  }, [])

  useEffect(() => {
    let active = true
    setSettingsLoading(true)
    setError('')
    fetchLatestSettings()
      .then(() => {
        if (active) {
          setStatus('Settings loaded. Click "See preview" to generate the latest email.')
        }
      })
      .catch((err: Error) => {
        if (active) {
          setError(err.message)
          setStatus('Unable to load settings')
        }
      })
      .finally(() => {
        if (active) {
          setSettingsLoading(false)
        }
      })
    return () => {
      active = false
    }
  }, [fetchLatestSettings])

  const handlePreview = useCallback(async () => {
    setError('')
    setPreviewLoading(true)
    setStatus('Generating preview…')
    try {
      const latestSettings = await fetchLatestSettings()
      const previewRes = await requestPreview(latestSettings)
      if (!previewRes.ok || !previewRes.data) {
        throw new Error(previewRes.error || 'preview failed')
      }
      setPreview(previewRes.data)
      setHasPreview(true)
      setStatus('Preview updated')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'preview failed')
      setStatus('Preview failed')
    } finally {
      setPreviewLoading(false)
    }
  }, [fetchLatestSettings])

  const handleSendTest = useCallback(async () => {
    if (!preview) return
    setSending(true)
    setError('')
    setStatus('')
    const recipient = settings?.userEmail
    try {
      const res = await sendTestEmail(recipient)
      if (res.ok) {
        setStatus('Test email sent')
      } else {
        setError(res.error || 'failed to send test email')
      }
    } catch (err: any) {
      setError(err?.message || 'failed to send test email')
    } finally {
      setSending(false)
    }
  }, [preview, settings])

  const isAdmin = user?.role === 'admin' || user?.role === 'dev'
  const previewButtonLabel = hasPreview ? 'Refresh preview' : 'See preview'

  return (
    <div className="box">
      <h2 className="section-title">Live Email Preview</h2>
      <p>Generate an on-demand snapshot of the email before dispatching it.</p>
      <div className="actions">
        <button
          type="button"
          onClick={handlePreview}
          disabled={settingsLoading || previewLoading}
        >
          {previewLoading ? 'Generating…' : previewButtonLabel}
        </button>
        {isAdmin && (
          <button
            type="button"
            onClick={handleSendTest}
            disabled={previewLoading || sending || !preview}
            className="button ghost"
          >
            {sending ? 'Sending…' : 'Send test email'}
          </button>
        )}
      </div>
      {(settingsLoading || previewLoading) && <div className="status">Working…</div>}
      {error && <div className="status error">{error}</div>}
      {!preview && !settingsLoading && !previewLoading && (
        <div className="status">{status}</div>
      )}
      {preview && !previewLoading && !error && (
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
