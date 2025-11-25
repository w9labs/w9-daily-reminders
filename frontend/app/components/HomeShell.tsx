'use client'

import SettingsForm from './SettingsForm'

export default function HomeShell() {
  return (
    <div className="box">
      <h2 className="section-title">Configure · Delivery Window</h2>
      <p>Set language, timezone, and AI guidance for the generated reminder mail.</p>
      <SettingsForm />
    </div>
  )
}
