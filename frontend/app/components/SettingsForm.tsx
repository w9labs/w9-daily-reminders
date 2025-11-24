'use client'

import { useEffect, useMemo, useState, useTransition } from 'react'
import { reminderSettingsSchema } from '../../lib/schemas'
import { DEFAULT_SETTINGS, LANGUAGES, SUMMARY_STYLES } from '../../lib/constants'
import { TIMEZONES } from '../../lib/timezones'
import type { ReminderPreview, ReminderSettings } from '../../lib/types'
import { getSettings, requestPreview, saveSettings, startGoogleAuth } from '../../lib/api'

interface Props {
  onPreview(preview: ReminderPreview): void
}

const customOptionValue = 'custom'

const deriveLanguageChoice = (language: string, customLanguage?: string): {
  choice: 'preset' | 'custom'
  normalizedCustom?: string
} => {
  if (customLanguage && customLanguage.trim().length > 0) {
    return { choice: 'custom', normalizedCustom: customLanguage }
  }
  if (!LANGUAGES.includes(language)) {
    return { choice: 'custom', normalizedCustom: language }
  }
  return { choice: 'preset' }
}

export default function SettingsForm({ onPreview }: Props) {
  const initialLanguage = deriveLanguageChoice(DEFAULT_SETTINGS.language)
  const [settings, setSettings] = useState<ReminderSettings>({
    userEmail: '',
    ...DEFAULT_SETTINGS,
    customLanguage: initialLanguage.normalizedCustom,
  })
  const [languageChoice, setLanguageChoice] = useState<'preset' | 'custom'>(initialLanguage.choice)
  const [saving, setSaving] = useState(false)
  const [status, setStatus] = useState<string>('waiting for configuration')
  const [error, setError] = useState<string>('')
  const [isPending, startTransition] = useTransition()

  useEffect(() => {
    let mounted = true
    getSettings().then((res) => {
      if (!mounted) return
      if (res.ok && res.data) {
        const normalized = deriveLanguageChoice(res.data.language, res.data.customLanguage ?? undefined)
        setSettings((prev) => ({
          ...prev,
          ...res.data,
          customLanguage: normalized.normalizedCustom,
        }))
        setLanguageChoice(normalized.choice)
        setStatus('restored saved configuration')
      }
    })
    return () => {
      mounted = false
    }
  }, [])

  const resolvedLanguage = useMemo(() => {
    if (languageChoice === 'custom') {
      return settings.customLanguage && settings.customLanguage.trim().length > 0
        ? settings.customLanguage
        : settings.language
    }
    return settings.language
  }, [languageChoice, settings.language, settings.customLanguage])

  const update = <K extends keyof ReminderSettings>(key: K, value: ReminderSettings[K]) => {
    setSettings((prev) => ({ ...prev, [key]: value }))
  }

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    setError('')
    setStatus('saving configuration')
    const payload = { ...settings, language: resolvedLanguage }
    const parse = reminderSettingsSchema.safeParse(payload)
    if (!parse.success) {
      setError(parse.error.errors[0]?.message || 'invalid form payload')
      setStatus('validation failed')
      return
    }

    try {
      setSaving(true)
      const response = await saveSettings(parse.data)
      if (!response.ok || !response.data) {
        setError(response.error || 'save failed')
        setStatus('save failed')
        return
      }
      setStatus('configuration saved · generating preview')
      const previewPayload = response.data ?? parse.data
      startTransition(() =>
        requestPreview(previewPayload).then((preview) => {
          if (preview.ok && preview.data) {
            onPreview(preview.data)
            setStatus('preview refreshed')
          } else {
            setError(preview.error || 'preview failed')
            setStatus('preview failed')
          }
        })
      )
    } catch (err) {
      setError(err instanceof Error ? err.message : 'unknown error')
      setStatus('exception')
    } finally {
      setSaving(false)
    }
  }

  const handleGoogleSync = async () => {
    const res = await startGoogleAuth()
    if (res.ok && res.data?.url) {
      window.open(res.data.url, '_blank')
      setStatus('google auth started in new tab')
    } else {
      setError(res.error || 'unable to start oauth')
    }
  }

  return (
    <form className="form" onSubmit={handleSubmit}>
      <div className="row">
        <label htmlFor="email">Primary email</label>
        <input
          id="email"
          type="email"
          value={settings.userEmail}
          onChange={(event) => update('userEmail', event.target.value)}
          placeholder="operator@w9.nu"
          required
        />
      </div>

      <div className="grid-two">
        <div className="row">
          <label htmlFor="time">Reminder time · local</label>
          <input
            id="time"
            type="time"
            value={settings.reminderTime}
            onChange={(event) => update('reminderTime', event.target.value)}
            required
          />
        </div>
        <div className="row">
          <label htmlFor="timezone">Timezone</label>
          <select
            id="timezone"
            value={settings.timezone}
            onChange={(event) => update('timezone', event.target.value)}
          >
            {TIMEZONES.map((zone) => (
              <option key={zone} value={zone}>
                {zone}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div className="row">
        <label htmlFor="language">Language</label>
        <select
          id="language"
          value={languageChoice === 'custom' ? customOptionValue : settings.language}
          onChange={(event) => {
            const nextValue = event.target.value
            if (nextValue === customOptionValue) {
              setLanguageChoice('custom')
              update('customLanguage', settings.customLanguage || settings.language)
            } else {
              setLanguageChoice('preset')
              update('language', nextValue)
              update('customLanguage', undefined)
            }
          }}
        >
          {LANGUAGES.map((lang) => (
            <option key={lang} value={lang}>
              {lang}
            </option>
          ))}
          <option value={customOptionValue}>Custom</option>
        </select>
        {languageChoice === 'custom' && (
          <input
            type="text"
            value={settings.customLanguage || ''}
            placeholder="Type language name"
            onChange={(event) => update('customLanguage', event.target.value)}
            required
          />
        )}
      </div>

      <div className="row">
        <label htmlFor="weather">Weather location · city or lat,long</label>
        <input
          id="weather"
          type="text"
          value={settings.weatherLocation}
          onChange={(event) => update('weatherLocation', event.target.value)}
          placeholder="Stockholm, Sweden"
          required
        />
        <small>Used to warn about umbrella, wind, frost, or heat advisories.</small>
      </div>

      <div className="row">
        <label>Inclusions</label>
        <div className="actions">
          <label className="nav-link" style={{ cursor: 'pointer' }}>
            <input
              type="checkbox"
              checked={settings.includeWeather}
              onChange={(event) => update('includeWeather', event.target.checked)}
            />
            &nbsp;Weather insight
          </label>
          <label className="nav-link" style={{ cursor: 'pointer' }}>
            <input
              type="checkbox"
              checked={settings.includeImage}
              onChange={(event) => update('includeImage', event.target.checked)}
            />
            &nbsp;Pollinations visual
          </label>
        </div>
      </div>

      <div className="row">
        <label>Summary voice</label>
        <div className="actions">
          {SUMMARY_STYLES.map((style) => (
            <label key={style.id} className="nav-link" style={{ cursor: 'pointer' }}>
              <input
                type="radio"
                name="summaryStyle"
                value={style.id}
                checked={settings.summaryStyle === style.id}
                onChange={() => update('summaryStyle', style.id as ReminderSettings['summaryStyle'])}
              />
              &nbsp;{style.label}
            </label>
          ))}
        </div>
      </div>

      <div className="actions">
        <button type="button" onClick={handleGoogleSync} className="button ghost" disabled={saving}>
          Sync Google Calendar
        </button>
        <button type="submit" disabled={saving || isPending}>
          {saving ? 'Saving…' : 'Save + Preview'}
        </button>
      </div>

      <div className={`status ${error ? 'error' : ''}`}>
        {error ? error : status}
      </div>
    </form>
  )
}
