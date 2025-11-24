'use client'

import { FormEvent, useState } from 'react'
import { useSearchParams } from 'next/navigation'
import TurnstileWidget from '../components/Turnstile'
import { MAIL_API_BASE } from '../../lib/config'

export default function ResetPasswordPage() {
  const searchParams = useSearchParams()
  const tokenParam = searchParams.get('token')
  const [email, setEmail] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [status, setStatus] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [turnstileToken, setTurnstileToken] = useState<string | null>(null)

  async function requestReset(event: FormEvent) {
    event.preventDefault()
    setError(null)
    setStatus(null)
    setLoading(true)
    try {
      const resp = await fetch(`${MAIL_API_BASE}/auth/password-reset`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, turnstile_token: turnstileToken ?? undefined }),
      })
      const data = await resp.json()
      if (!resp.ok) {
        throw new Error(data.message || data.error || 'Reset failed')
      }
      setStatus(data.message || 'If the email exists, a reset link was sent.')
    } catch (err: any) {
      setError(err?.message || 'Reset failed')
    } finally {
      setLoading(false)
    }
  }

  async function confirmReset(event: FormEvent) {
    event.preventDefault()
    if (!tokenParam) return
    setError(null)
    setStatus(null)
    setLoading(true)
    try {
      const resp = await fetch(`${MAIL_API_BASE}/auth/password-reset/confirm`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ token: tokenParam, newPassword: newPassword, turnstile_token: turnstileToken ?? undefined }),
      })
      const data = await resp.json()
      if (!resp.ok) {
        throw new Error(data.message || data.error || 'Reset failed')
      }
      setStatus(data.message || 'Password updated. You can sign in now.')
    } catch (err: any) {
      setError(err?.message || 'Reset failed')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="box">
      <h2 className="section-title">Password reset</h2>
      {!tokenParam ? (
        <form className="form" onSubmit={requestReset}>
          <label className="row">
            Account email
            <input type="email" value={email} onChange={(e) => setEmail(e.target.value)} required />
          </label>
          <TurnstileWidget onVerify={(token) => setTurnstileToken(token)} onError={() => setTurnstileToken(null)} />
          <button type="submit" disabled={loading}>
            {loading ? 'Sending…' : 'Send reset link'}
          </button>
          {status && <div className="status success">{status}</div>}
          {error && <div className="status error">{error}</div>}
        </form>
      ) : (
        <form className="form" onSubmit={confirmReset}>
          <p className="hint">Reset token detected. Set a new password below.</p>
          <label className="row">
            New password
            <input type="password" value={newPassword} onChange={(e) => setNewPassword(e.target.value)} minLength={8} required />
          </label>
          <TurnstileWidget onVerify={(token) => setTurnstileToken(token)} onError={() => setTurnstileToken(null)} />
          <button type="submit" disabled={loading}>
            {loading ? 'Updating…' : 'Update password'}
          </button>
          {status && <div className="status success">{status}</div>}
          {error && <div className="status error">{error}</div>}
        </form>
      )}
    </div>
  )
}
