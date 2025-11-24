'use client'

import { FormEvent, useState } from 'react'
import TurnstileWidget from '../components/Turnstile'
import { MAIL_API_BASE } from '../../lib/config'

export default function RegisterPage() {
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [confirm, setConfirm] = useState('')
  const [status, setStatus] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [turnstileToken, setTurnstileToken] = useState<string | null>(null)

  async function handleRegister(event: FormEvent) {
    event.preventDefault()
    setError(null)
    setStatus(null)
    if (password !== confirm) {
      setError('Passwords do not match')
      return
    }
    setLoading(true)
    try {
      const resp = await fetch(`${MAIL_API_BASE}/auth/signup`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, password, turnstile_token: turnstileToken ?? undefined }),
      })
      const data = await resp.json()
      if (!resp.ok) {
        throw new Error(data.message || data.error || 'Registration failed')
      }
      setStatus(data.message || 'Registration successful. Check your inbox to verify your email.')
    } catch (err: any) {
      setError(err?.message || 'Registration failed')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="box">
      <h2 className="section-title">Register</h2>
      <form className="form" onSubmit={handleRegister}>
        <label className="row">
          Email
          <input type="email" value={email} onChange={(e) => setEmail(e.target.value)} required />
        </label>
        <label className="row">
          Password
          <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} minLength={8} required />
        </label>
        <label className="row">
          Confirm password
          <input type="password" value={confirm} onChange={(e) => setConfirm(e.target.value)} minLength={8} required />
        </label>
        <TurnstileWidget onVerify={(token) => setTurnstileToken(token)} onError={() => setTurnstileToken(null)} />
        <button type="submit" disabled={loading}>
          {loading ? 'Submitting…' : 'Create account'}
        </button>
        {status && <div className="status success">{status}</div>}
        {error && <div className="status error">{error}</div>}
      </form>
    </div>
  )
}
