import { DateTime } from 'luxon'

export function formatTime(time: string, timezone: string) {
  const [hour, minute] = time.split(':').map(Number)
  return DateTime.fromObject({ hour, minute }, { zone: timezone }).toFormat('hh:mm a ZZZZ')
}

export function isoNow(timezone: string) {
  return DateTime.now().setZone(timezone).toISO()
}
