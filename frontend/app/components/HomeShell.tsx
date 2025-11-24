'use client'

import { useState } from 'react'
import type { ReminderPreview } from '../../lib/types'
import SettingsForm from './SettingsForm'
import EmailPreview from './EmailPreview'

export default function HomeShell() {
  const [preview, setPreview] = useState<ReminderPreview>()

  return (
    <div className="grid-two">
      <div className="box">
        <h2 className="section-title">Configure · Delivery Window</h2>
        <p>Set language, timezone, and AI guidance for the generated reminder mail.</p>
        <SettingsForm onPreview={setPreview} />
      </div>
      <div className="box">
        <h2 className="section-title">Preview · Email Frame</h2>
        <EmailPreview preview={preview} />
      </div>
    </div>
  )
}
