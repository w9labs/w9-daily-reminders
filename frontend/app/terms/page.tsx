const serviceBullets = [
  'Sync your Google Calendar + Tasks in day or week mode with configurable week starts.',
  'Blend hourly or weekly weather guidance into the Retro Astrological Calendar Card template.',
  'Generate subject, preview, HTML, text and hero art via Cerebras + Pollinations/Cloudflare with strict JSON schemas.',
  'Deliver through W9 Mail with inline CID assets so Gmail and Outlook show banners reliably.',
]

const responsibilities = [
  'Maintain secure credentials (enable Turnstile, rotate passwords, keep env secrets private).',
  'Review AI output before forwarding to others; you control the final send.',
  'Respect Google API quotas and terms when connecting shared calendars or task lists.',
  'Avoid injecting PHI/PCI or other regulated data unless your hosting environment is compliant.',
]

const disclaimers = [
  'Service is provided “as is.” Third-party outages (Google, Cerebras, Cloudflare, weather providers) can delay reminders.',
  'AI copy may hallucinate. Always skim before sending.',
  'Email delivery depends on W9 Mail or your SMTP config; we cannot guarantee inbox placement.',
]

export default function TermsPage() {
  return (
    <>
      <div className="box">
        <h1>Terms of Service</h1>
        <p className="subtitle">W9 Daily Reminders is a W9 Labs project. Using it means you accept these rules.</p>
      </div>
      <div className="box">
        <h2 className="section-title">What The Service Does</h2>
        <ul className="list">
          {serviceBullets.map((item) => (
            <li key={item}>{item}</li>
          ))}
        </ul>
      </div>
      <div className="box">
        <h2 className="section-title">Your Responsibilities</h2>
        <ul className="list">
          {responsibilities.map((item) => (
            <li key={item}>{item}</li>
          ))}
        </ul>
      </div>
      <div className="box">
        <h2 className="section-title">Availability & Disclaimers</h2>
        <ul className="list">
          {disclaimers.map((item) => (
            <li key={item}>{item}</li>
          ))}
        </ul>
      </div>
      <div className="box">
        <h2 className="section-title">Data & Privacy</h2>
        <p>
          Refer to our <a href="/privacy">Privacy Notice</a> for details on storage and deletion. We keep data on the server you run and
          only share it with the providers listed there.
        </p>
      </div>
      <div className="box">
        <h2 className="section-title">Termination</h2>
        <p>
          Admins may disable accounts or revoke Google tokens at any time. W9 Labs can suspend managed instances for abuse or security
          concerns. You can delete your own account and cached previews through the UI.
        </p>
      </div>
      <div className="box">
        <h2 className="section-title">Updates</h2>
        <p>
          Terms evolve as dependencies (Cerebras, Cloudflare, Google) change their policies. We’ll document significant updates in the
          repo changelog. Continued use equals acceptance.
        </p>
      </div>
      <div className="box">
        <h2 className="section-title">Contact</h2>
        <p>
          Email <a href="mailto:hi@w9.se">hi@w9.se</a> for legal questions. W9 Labs is a non-profit collective operating across the EU/EEA.
        </p>
      </div>
    </>
  )
}

