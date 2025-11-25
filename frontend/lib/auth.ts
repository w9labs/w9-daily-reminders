'use client'

const TOKEN_KEY = 'w9_token'

export const AUTH_EVENT = 'w9:auth'

function dispatchAuthEvent() {
  if (typeof window === 'undefined') return
  window.dispatchEvent(new Event(AUTH_EVENT))
}

export function getStoredToken(): string | null {
  if (typeof window === 'undefined') return null
  return window.localStorage.getItem(TOKEN_KEY)
}

export function storeToken(token: string) {
  if (typeof window === 'undefined') return
  window.localStorage.setItem(TOKEN_KEY, token)
  dispatchAuthEvent()
}

export function clearToken() {
  if (typeof window === 'undefined') return
  window.localStorage.removeItem(TOKEN_KEY)
  dispatchAuthEvent()
}
