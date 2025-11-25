import { timeZonesNames } from '@vvo/tzdb'

export const TIMEZONES: string[] = Array.isArray(timeZonesNames) && timeZonesNames.length > 0 ? timeZonesNames : [
  'UTC',
  'Europe/London',
  'Europe/Stockholm',
  'America/New_York',
  'America/Los_Angeles',
  'Asia/Tokyo',
  'Asia/Ho_Chi_Minh_City',
  'Australia/Sydney',
]
