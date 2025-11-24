'use client'

import { Suspense, useEffect, useMemo, useState } from 'react'
import { useSearchParams, useRouter } from 'next/navigation'
import { MAIL_API_BASE } from '../../../lib/config'
import { storeToken } from '../../../lib/auth'

function VerifyContent() {
  const searchParams = useSearchParams()
  const token = useMemo(() => searchParams.get('token'), [searchParams])
  const router = useRouter()
  const [status, setStatus] = useState<'pending' | 'success' | 'error'>('pending')
  const [message, setMessage] = useState('Verifying your email token…')

  useEffect(() => {
    if (!token) {
      setStatus('error')
      setMessage('Missing verification token')
      return
    }

    let cancelled = false
    async function verify() {
      try {
        const resp = await fetch(`${MAIL_API_BASE}/auth/signup/verify`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ token }),
        })
        const data = await resp.json()
        if (cancelled) return
        if (!resp.ok) {
          throw new Error(data.message || data.error || 'Verification failed')
        }
        if (data.token) {
          storeToken(data.token)
        }
        setStatus('success')
        setMessage(data.message || 'Email verified. Redirecting to dashboard…')
        setTimeout(() => router.push('/'), 1500)
      } catch (err: any) {
        if (cancelled) return
        setStatus('error')
        setMessage(err?.message || 'Verification failed')
      }
    }

    verify()
    return () => {
      cancelled = true
    }
  }, [router, token])

  return (
    <div className="box">
      <h2 className="section-title">Email verification</h2>
      <div className={`status ${status === 'error' ? 'error' : status === 'success' ? 'success' : ''}`}>{message}</div>
    </div>
  )
}

export default function VerifyPage() {
  return (
    <Suspense fallback={<div className="box">Email verification · Loading…</div>}>
      <VerifyContent />
    </Suspense>
  )
}
