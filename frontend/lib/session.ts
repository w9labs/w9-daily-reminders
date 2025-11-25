'use client'

import { useEffect, useState, useCallback } from 'react'
import { MAIL_API_BASE } from './config'
import { clearToken, getStoredToken } from './auth'

export interface SessionUser {
  id: string
  email: string
  role: string
  mustChangePassword?: boolean
}

export function useSession() {
  const [user, setUser] = useState<SessionUser | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    const token = getStoredToken()
    if (!token) {
      setUser(null)
      setLoading(false)
      return
    }
    try {
      setLoading(true)
      const resp = await fetch(`${MAIL_API_BASE}/auth/me`, {
        headers: {
          Authorization: `Bearer ${token}`,
        },
        cache: 'no-store',
      })
      if (!resp.ok) {
        throw new Error('Unable to load session')
      }
      const data = await resp.json()
      setUser({
        id: data.id,
        email: data.email,
        role: data.role,
        mustChangePassword: data.mustChangePassword,
      })
      setError(null)
    } catch (err: any) {
      setError(err?.message || 'Session error')
      clearToken()
      setUser(null)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    refresh()

    const handleAuthChange = () => refresh()
    window.addEventListener('w9:auth', handleAuthChange)
    return () => window.removeEventListener('w9:auth', handleAuthChange)
  }, [refresh])

  const logout = useCallback(() => {
    clearToken()
    setUser(null)
  }, [])

  return { user, loading, error, refresh, logout }
}
