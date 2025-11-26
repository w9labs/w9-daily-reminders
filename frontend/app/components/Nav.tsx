'use client'

import Link from 'next/link'
import { usePathname, useRouter } from 'next/navigation'
import { useSession } from '../../lib/session'
import { clearToken } from '../../lib/auth'

export default function Nav() {
  const pathname = usePathname()
  const { user } = useSession()
  const router = useRouter()

  const publicLinks = [
    { href: '/', label: 'Console' },
    { href: '/preview', label: 'Preview' },
    { href: '/system', label: 'System' },
  ]

  const authLinks =
    user
      ? [
          { href: '/admin', label: 'Admin', type: 'link' as const },
          { href: '#logout', label: 'Sign out', type: 'action' as const },
        ]
      : [
          { href: '/login', label: 'Login', type: 'link' as const },
          { href: '/register', label: 'Register', type: 'link' as const },
        ]

  return (
    <nav className="nav">
      {publicLinks.map((link) => (
        <Link key={link.href} href={link.href} className={`nav-link ${pathname === link.href ? 'active' : ''}`}>
          {link.label}
        </Link>
      ))}
      {authLinks.map((link) =>
        link.type === 'link' ? (
          <Link key={link.href} href={link.href} className={`nav-link ${pathname === link.href ? 'active' : ''}`}>
            {link.label}
          </Link>
        ) : (
          <button
            key={link.label}
            type="button"
            className="nav-link"
            onClick={() => {
              clearToken()
              router.push('/login')
            }}
          >
            {link.label}
          </button>
        ),
      )}
    </nav>
  )
}

