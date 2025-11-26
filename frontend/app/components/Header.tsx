'use client'

import Link from 'next/link'
import { useRouter } from 'next/navigation'
import { useSession } from '../../lib/session'
import { clearToken } from '../../lib/auth'
import NetworkBar from './NetworkBar'

export default function Header() {
  const { user } = useSession()
  const router = useRouter()

  return (
    <>
      <NetworkBar active="reminders" />
      <header className="header">
        <div className="header-top">
          <div>
            <p className="eyebrow">Developed by W9 Labs</p>
            <h1>W9 Daily Reminders</h1>
            <p>Google Calendar sync · Cerebras copy · Pollinations + Cloudflare visuals</p>
          </div>
          <div className="session">
            {user ? (
              <>
                <div
                  className="pill"
                  style={{ borderColor: '#00ffd0', color: '#00ffd0' }}
                >
                  SIGNED IN
                </div>
              </>
            ) : (
              <div className="actions">
                <div className="pill">GUEST</div>
              </div>
            )}
          </div>
        </div>
      </header>
    </>
  )
}
