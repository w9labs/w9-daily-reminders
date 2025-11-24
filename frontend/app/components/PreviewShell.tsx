'use client'

import { useCallback, useEffect, useState } from 'react'
import type { ReminderPreview, ReminderSettings } from '../../lib/types'
import { getSettings, requestPreview } from '../../lib/api'

export default function PreviewShell() {
  const [settings, setSettings] = useState<ReminderSettings>()
  const [preview, setPreview] = useState<ReminderPreview>()
  const [status, setStatus] = useState('bootstrap')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)

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

  useEffect(() => {
    hydrate()
  }, [hydrate])

  return (
    <div className="box">
      <h2 className="section-title">Live Email Preview</h2>
      <p>Renders the exact HTML + plaintext copy currently scheduled for delivery.</p>
      <div className="actions">
        <button type="button" onClick={hydrate} disabled={loading}>
          Refresh preview
        </button>
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
