export const LANGUAGES = ['English', 'Svenska', 'Deutsch', 'Español', 'Português', '日本語']

export const SUMMARY_STYLES = [
  { id: 'concise', label: 'Concise · Operative bullets' },
  { id: 'detailed', label: 'Detailed · Narrative brief' },
  { id: 'bullet', label: 'Bullet · Checklist voice' },
]

import type { ReminderSettings } from './types'

export const DEFAULT_SETTINGS: Omit<ReminderSettings, 'userEmail'> = {
  reminderTime: '07:30',
  timezone: 'Europe/Stockholm',
  language: 'English',
  weatherLocation: 'Stockholm, Sweden',
  includeWeather: true,
  includeImage: true,
  imageProvider: 'pollinations',
  cloudflareModel: '@cf/black-forest-labs/flux-2-dev',
  summaryStyle: 'concise',
}

