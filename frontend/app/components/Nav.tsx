'use client'

import Link from 'next/link'
import { usePathname } from 'next/navigation'
import { useSession } from '../../lib/session'

export default function Nav() {
  const pathname = usePathname()
  const { user } = useSession()

  const publicLinks = [
    { href: '/', label: 'Console' },
    { href: '/preview', label: 'Preview' },
    { href: '/system', label: 'System' },
  ]

  const authLinks = user
    ? [{ href: '/admin', label: 'Admin' }]
    : [
        { href: '/login', label: 'Login' },
        { href: '/register', label: 'Register' },
      ]

  return (
    <nav className="nav">
      {publicLinks.map((link) => (
        <Link key={link.href} href={link.href} className={`nav-link ${pathname === link.href ? 'active' : ''}`}>
          {link.label}
        </Link>
      ))}
      {authLinks.map((link) => (
        <Link key={link.href} href={link.href} className={`nav-link ${pathname === link.href ? 'active' : ''}`}>
          {link.label}
        </Link>
      ))}
    </nav>
  )
}

