'use client'

import { FormEvent, useState } from 'react'
import { useRouter } from 'next/navigation'
import TurnstileWidget from '../components/Turnstile'
import { MAIL_API_BASE } from '../../lib/config'
import { storeToken } from '../../lib/auth'

export default function LoginPage() {
  const router = useRouter()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [status, setStatus] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [turnstileToken, setTurnstileToken] = useState<string | null>(null)

  async function handleLogin(event: FormEvent) {
    event.preventDefault()
    setError(null)
    setStatus(null)
    setLoading(true)
    try {
      const resp = await fetch(`${MAIL_API_BASE}/auth/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email, password, turnstile_token: turnstileToken ?? undefined }),
      })
      const data = await resp.json()
      if (!resp.ok) {
        throw new Error(data.message || data.error || 'Login failed')
      }
      if (data.token) {
        storeToken(data.token)
        setStatus('Login successful · redirecting')
        router.push('/')
      } else {
        throw new Error('Missing token in response')
      }
    } catch (err: any) {
      setError(err?.message || 'Login failed')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="box">
      <h2 className="section-title">Login</h2>
      <form className="form" onSubmit={handleLogin}>
        <label className="row">
          Email
          <input type="email" value={email} onChange={(e) => setEmail(e.target.value)} required />
        </label>
        <label className="row">
          Password
          <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} required />
        </label>
        <TurnstileWidget onVerify={(token) => setTurnstileToken(token)} onError={() => setTurnstileToken(null)} />
        <button type="submit" disabled={loading}>
          {loading ? 'Signing in…' : 'Sign in'}
        </button>
        {status && <div className="status success">{status}</div>}
        {error && <div className="status error">{error}</div>}
      </form>
    </div>
  )
}
