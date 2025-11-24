import SystemStatus from './SystemStatus'

export default function SystemShell() {
  return (
    <>
      <div className="box">
        <h2 className="section-title">System · Deployment Checklist</h2>
      <p>Axum orchestrator exposes strict JSON endpoints consumed by this frontend.</p>
      <ul className="list">
        <li>Configure env vars via install.sh (CEREBRAS_API_KEY, GOOGLE_CLIENT_ID/SECRET).</li>
        <li>Backend listens on :8787; Next proxy uses NEXT_PUBLIC_API_BASE.</li>
        <li>install.sh builds frontend with `npm run build` and deploys static output via PM2/Next.</li>
      </ul>
      <div className="table-wrapper">
        <table>
          <thead>
            <tr>
              <th>Endpoint</th>
              <th>Purpose</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>/api/google/start</td>
              <td>Initiate OAuth with Google Calendar</td>
            </tr>
            <tr>
              <td>/api/google/callback</td>
              <td>Store tokens and refresh schedules</td>
            </tr>
            <tr>
              <td>/api/settings</td>
              <td>Persist reminder configuration</td>
            </tr>
            <tr>
              <td>/api/reminders/preview</td>
              <td>Call Cerebras + weather + Pollinations for HTML</td>
            </tr>
            <tr>
              <td>/api/system/health</td>
              <td>Scheduler heartbeat</td>
            </tr>
          </tbody>
        </table>
      </div>
      <div className="box">
        <h2 className="section-title">Orchestrator health</h2>
        <SystemStatus />
      </div>
    </>
  )
}
