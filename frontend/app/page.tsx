import HomeShell from './components/HomeShell'
import SystemStatus from './components/SystemStatus'

export default function Page() {
  return (
    <>
      <div className="box">
        <h2 className="section-title">Daily calendar briefings, in one email</h2>
        <p>
          W9 Daily Reminders takes your Google Calendar and Tasks, local weather, and a static creative brief and turns it into a single
          beautifully formatted email in the Retro Astrological Calendar Card layout. It runs on your own infrastructure, using W9 Mail
          to deliver HTML that matches the preview exactly.
        </p>
        <ul className="list">
          <li>
            <strong>Day or week schedule</strong>: Choose between a focused day view with 4‑hour weather slices, or a weekly digest with
            per‑day advisories (umbrella, coat, sunscreen, etc.).
          </li>
          <li>
            <strong>Calendar + Tasks together</strong>: Syncs Google Calendar events and Google Tasks, grouping them by day so deadlines,
            meetings, and todos show up in the same narrative.
          </li>
          <li>
            <strong>Cerebras‑powered copy</strong>: Uses Cerebras models with strict JSON schemas to generate subject, preview text,
            HTML body, plain text, and a safe hero prompt in one call.
          </li>
          <li>
            <strong>Cloudflare + Pollinations visuals</strong>: Pick Pollinations or Workers AI (flux‑1‑schnell, flux‑2‑dev, SDXL, etc.);
            prompts are sanitized to avoid copyright and moderation issues.
          </li>
          <li>
            <strong>Exact preview = sent HTML</strong>: Every preview is cached and reused for test sends so what you see in the UI is
            exactly what arrives in the inbox, including inline CID images.
          </li>
          <li>
            <strong>W9 Labs stack</strong>: Rust + Axum backend, Next.js admin UI, Cerebras + Cloudflare integrations, and W9 Mail
            delivery—maintained as a W9 Labs open‑source project.
          </li>
        </ul>
      </div>
      <HomeShell />
      <div className="box">
        <h2 className="section-title">Orchestrator health</h2>
        <SystemStatus />
      </div>
    </>
  )
}
