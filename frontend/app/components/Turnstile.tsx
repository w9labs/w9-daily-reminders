'use client'

import { useEffect, useRef } from 'react'
import { TURNSTILE_SITE_KEY } from '../../lib/config'

declare global {
  interface Window {
    turnstile?: {
      render: (
        element: HTMLElement,
        options: {
          sitekey: string
          theme?: 'light' | 'dark'
          callback?: (token: string) => void
          'error-callback'?: () => void
        }
      ) => string
      reset?: (widgetId?: string) => void
    }
  }
}

interface TurnstileWidgetProps {
  onVerify?: (token: string) => void
  onError?: () => void
}

export default function TurnstileWidget({ onVerify, onError }: TurnstileWidgetProps) {
  const ref = useRef<HTMLDivElement | null>(null)
  const scriptInjected = useRef(false)
  const latestVerify = useRef(onVerify)
  const latestError = useRef(onError)

  useEffect(() => {
    latestVerify.current = onVerify
  }, [onVerify])

  useEffect(() => {
    latestError.current = onError
  }, [onError])

  useEffect(() => {
    if (!TURNSTILE_SITE_KEY || !ref.current) {
      return
    }

    let widgetId: string | undefined

    function renderWidget() {
      if (!window.turnstile || !ref.current) return
      if (ref.current.firstChild) {
        ref.current.innerHTML = ''
      }
      const id = window.turnstile.render(ref.current, {
        sitekey: TURNSTILE_SITE_KEY,
        theme: 'dark',
        callback: (token: string) => latestVerify.current?.(token),
        'error-callback': () => latestError.current?.(),
      })
      widgetId = id
    }

    const existing = document.querySelector('script[src*="challenges.cloudflare.com/turnstile"]')

    if (window.turnstile) {
      renderWidget()
    } else if (existing) {
      existing.addEventListener('load', renderWidget, { once: true })
    } else if (!scriptInjected.current) {
      scriptInjected.current = true
      const script = document.createElement('script')
      script.src = 'https://challenges.cloudflare.com/turnstile/v0/api.js'
      script.async = true
      script.defer = true
      script.onload = () => renderWidget()
      script.onerror = () => latestError.current?.()
      document.head.appendChild(script)
    }

    return () => {
      if (widgetId && window.turnstile?.reset) {
        window.turnstile.reset(widgetId)
      }
    }
  }, [])

  if (!TURNSTILE_SITE_KEY) {
    return null
  }

  return <div ref={ref} data-testid="turnstile-widget" />
}
