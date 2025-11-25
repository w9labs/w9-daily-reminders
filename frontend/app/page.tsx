import HomeShell from './components/HomeShell'
import SystemStatus from './components/SystemStatus'

export default function Page() {
  return (
    <>
      <div className="box">
        <h2 className="section-title">Mission</h2>
        <p>
          W9 Daily Reminders is a production-grade orchestration service that transforms calendar data into intelligent, 
          context-aware daily briefings. By seamlessly integrating Google Calendar synchronization, AI-powered content 
          generation, and dynamic visual storytelling, we deliver personalized reminders that help users stay organized 
          and informed.
        </p>
        <ul className="list">
          <li>
            <strong>Enterprise-Grade Architecture</strong>: Built on Rust (Axum) and Next.js with secure OAuth2 integration, 
            role-based access control, and production-ready deployment automation.
          </li>
          <li>
            <strong>AI-Powered Intelligence</strong>: Leverages Cerebras zai-glm-4.6 for context-aware email generation, 
            ensuring each briefing is tailored to your schedule, preferences, and local context.
          </li>
          <li>
            <strong>Visual Storytelling</strong>: Pollinations.ai integration with model selection and intelligent caching 
            generates unique visual cues that complement your daily agenda.
          </li>
          <li>
            <strong>Weather Intelligence</strong>: Location-based weather advisories provide actionable insights for 
            your day, from umbrella reminders to heat warnings.
          </li>
          <li>
            <strong>W9 Mail Integration</strong>: Unified authentication, user management, and reliable email delivery 
            through the W9 Mail infrastructure.
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
