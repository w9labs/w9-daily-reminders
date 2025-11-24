'use client'

import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { useSession } from '../../lib/session'
import { clearToken } from '../../lib/auth'

const links = [
  { href: '/', label: 'Console' },
  { href: '/preview', label: 'Preview' },
  { href: '/system', label: 'System' },
  { href: '/admin', label: 'Admin' },
]

export default function Header() {
  const pathname = usePathname()
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
      <nav className="nav">
        {links.map((link) => (
          <Link key={link.href} href={link.href} className={`nav-link ${pathname === link.href ? 'active' : ''}`}>
            {link.label}
          </Link>
        ))}
        <Link href="/login" className={`nav-link ${pathname === '/login' ? 'active' : ''}`}>
          Login
        </Link>
        <Link href="/register" className={`nav-link ${pathname === '/register' ? 'active' : ''}`}>
          Register
        </Link>
      </nav>
    </header>
  )
}
