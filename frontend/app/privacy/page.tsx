const sections = [
  {
    title: 'What We Store',
    items: [
      'Account email + password hash.',
      'Reminder settings (schedule mode, timezone, weather location, provider preferences).',
      'Google OAuth tokens for Calendar and Tasks (encrypted at rest).',
      'Cached preview payloads so you can resend identical HTML without rerunning AI.',
    ],
  },
  {
    title: 'How Data Is Used',
    items: [
      'Fetch events and todos from Google Calendar/Tasks within the time window you configure.',
      'Generate copy with Cerebras models you choose and sanitize that output before emailing.',
      'Render hero imagery via Pollinations or Cloudflare Workers AI using the static creative brief.',
      'Deliver email through W9 Mail with inline CID assets so clients like Gmail display banners correctly.',
    ],
  },
  {
    title: 'Third Parties',
    items: [
      'Google APIs (Calendar + Tasks) — scoped to read-only access.',
      'Cerebras inference API for structured copy output.',
      'Cloudflare Workers AI or Pollinations for hero image generation.',
      'Open-Meteo (or configured weather provider) for forecast data.',
      'W9 Mail SMTP rail for final delivery.',
    ],
  },
  {
    title: 'Retention & Deletion',
    body:
      'Everything lives on the server you operate. Delete an account or disconnect Google to remove tokens and cached previews. Weather + AI responses are discarded after they are embedded into the outgoing email. System backups, if enabled by you, should be encrypted.',
  },
  {
    title: 'Security',
    body:
      'OAuth tokens are encrypted with per-install keys. Control characters in AI output are stripped before parsing to avoid injection. Admin routes require auth and rate limiting. Keep your .env secrets safe and enable HTTPS/Turnstile on the frontend.',
  },
  {
    title: 'Contact',
    body:
      'W9 Daily Reminders is a W9 Labs project. Reach the maintainers at hi@w9.se for privacy questions or to request export/delete support.',
  },
]

export default function PrivacyPage() {
  return (
    <>
      <div className="box">
        <h1>Privacy Notice</h1>
        <p className="subtitle">We only store what’s required to assemble your reminder email.</p>
      </div>
      {sections.map((section) => (
        <div className="box" key={section.title}>
          <h2 className="section-title">{section.title}</h2>
          {section.items ? (
            <ul className="list">
              {section.items.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
          ) : (
            <p>{section.body}</p>
          )}
        </div>
      ))}
    </>
  )
}

