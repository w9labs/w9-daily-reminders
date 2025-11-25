import { z } from 'zod'

export const reminderSettingsSchema = z.object({
  userEmail: z.string().email('valid email required'),
  reminderTime: z.string().regex(/^\d{2}:\d{2}$/),
  timezone: z.string().min(3),
  language: z.string(),
  customLanguage: z.string().optional(),
  weatherLocation: z.string().min(2),
  includeWeather: z.boolean(),
  includeImage: z.boolean(),
  imageModel: z.string().optional(),
  summaryStyle: z.enum(['concise', 'detailed', 'bullet']),
})

export type ReminderSettingsInput = z.infer<typeof reminderSettingsSchema>
