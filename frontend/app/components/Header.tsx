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
                <p>
                  Signed in as <span className="mono">{user.email}</span> · {user.role.toUpperCase()}
                </p>
                <div className="actions">
                  <button
                    type="button"
                    className="button ghost"
                    onClick={() => {
                      clearToken()
                      router.push('/login')
                    }}
                  >
                    Sign out
                  </button>
                </div>
              </>
            ) : (
              <div className="actions">
                <Link href="/login" className="button ghost">
                  Login
                </Link>
                <Link href="/register" className="button">
                  Register
                </Link>
              </div>
            )}
          </div>
        </div>
      </header>
    </>
  )
}
