export interface CalendarEvent {
  id: string
  summary: string
  start: string
  end: string
  location?: string
}

export interface ReminderSettings {
  userEmail: string
  reminderTime: string
  timezone: string
  language: string
  customLanguage?: string
  weatherLocation: string
  includeWeather: boolean
  includeImage: boolean
  imageModel?: string
  summaryStyle: 'concise' | 'detailed' | 'bullet'
}

export interface ReminderPreview {
  subject: string
  html: string
  text: string
  weatherAdvisory?: string
  imageUrl?: string
  generatedLanguage: string
}

export interface HealthStatus {
  scheduler: 'idle' | 'waiting' | 'sending'
  lastDispatch?: string
  nextRun?: string
  googleConnected: boolean
}

export interface ApiResponse<T> {
  ok: boolean
  data?: T
  error?: string
}
