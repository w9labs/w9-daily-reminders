'use client'

import { useSession } from '../../lib/session'

export default function ProfilePage() {
  const { user, loading, error, logout } = useSession()

  if (loading) {
    return <div className="box">Loading session…</div>
  }

  if (error) {
    return <div className="box">{error}</div>
  }

  if (!user) {
    return <div className="box">Not signed in. Visit the login page.</div>
  }

  return (
    <div className="box">
      <h2 className="section-title">Account</h2>
      <p>Email · {user.email}</p>
      <p>Role · {user.role.toUpperCase()}</p>
      {user.mustChangePassword && <div className="status warning">Password rotation required via W9 Mail</div>}
      <div className="actions">
        <button type="button" onClick={() => logout()}>
          Sign out
        </button>
        <a className="button ghost" href="/reset-password">
          Reset password
        </a>
      </div>
    </div>
  )
}
