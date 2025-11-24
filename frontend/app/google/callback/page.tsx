'use client'

import { useEffect, useState } from 'react'
import { useRouter, useSearchParams } from 'next/navigation'
import { completeGoogleAuth } from '../../../lib/api'

export default function GoogleCallbackPage() {
  const searchParams = useSearchParams()
  const router = useRouter()
  const [status, setStatus] = useState('Validating Google authorization code')
  const [error, setError] = useState('')

  useEffect(() => {
    const code = searchParams.get('code')
    if (!code) {
      setError('Missing code parameter')
      return
    }
    completeGoogleAuth(code).then((res) => {
      if (res.ok && res.data?.connected) {
        setStatus('Google Calendar connected. You may close this tab.')
        setTimeout(() => router.push('/'), 2500)
      } else {
        setError(res.error || 'Unable to complete Google OAuth')
      }
    })
  }, [searchParams, router])

  return (
    <div className="box">
      <h2 className="section-title">Google OAuth</h2>
      <p>Waiting for Google authorization callback.</p>
      <div className={`status ${error ? 'error' : ''}`}>{error || status}</div>
    </div>
  )
}
