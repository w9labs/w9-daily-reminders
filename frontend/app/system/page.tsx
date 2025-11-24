import SystemShell from '../components/SystemShell'

export default function SystemPage() {
  return (
    <>
      <header className="header">
        <h1>System · Ops Console</h1>
        <p>Deployment + scheduler diagnostics.</p>
      </header>
      <SystemShell />
    </>
  )
}
