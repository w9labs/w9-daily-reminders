'use client'

import Link from 'next/link'
import { useSession } from '../../lib/session'
import { clearToken } from '../../lib/auth'

export default function Header() {
  const { user } = useSession()

  return (
    <header className="header">
      <div className="header-top">
        <div>
          <h1>W9 Daily Reminders</h1>
          <p>Google Calendar sync · Cerebras copy · Pollinations visual</p>
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
                    window.location.href = '/login'
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
  )
}
