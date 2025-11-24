import HomeShell from './components/HomeShell'
import SystemStatus from './components/SystemStatus'

export default function Page() {
  return (
    <>
      <header className="header">
        <h1>W9 Daily Reminders · Console Briefings</h1>
        <p>Google Calendar sync + Cerebras zai-glm-4.6 reasoning + Pollinations art directive.</p>
      </header>
      <div className="box">
        <h2 className="section-title">Mission</h2>
        <p>
          Sync calendars, blend weather, and dispatch AI-generated schedules at the exact local hour you pick.
        </p>
        <ul className="list">
          <li>OAuth connect to Google Calendar; store tokens in the Axum orchestrator.</li>
          <li>
            Cerebras zai-glm-4.6 shapes the copy, while static HTML skeleton keeps branding identical to
            frontend-example.jpg.
          </li>
          <li>Pollinations image endpoint renders the visual cue for the day.</li>
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
