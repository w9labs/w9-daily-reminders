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

  useEffect(() => {
    if (!TURNSTILE_SITE_KEY || !ref.current) {
      return
    }
    let widgetId: string | undefined

    function renderWidget() {
      if (!window.turnstile || !ref.current) return
      const id = window.turnstile.render(ref.current, {
        sitekey: TURNSTILE_SITE_KEY,
        theme: 'dark',
        callback: (token: string) => onVerify?.(token),
        'error-callback': () => onError?.(),
      })
      widgetId = id
    }

    if (window.turnstile) {
      renderWidget()
    } else {
      const handler = () => renderWidget()
      window.addEventListener('turnstile-loaded', handler, { once: true })
      return () => window.removeEventListener('turnstile-loaded', handler)
    }

    return () => {
      if (widgetId && window.turnstile?.reset) {
        window.turnstile.reset(widgetId)
      }
    }
  }, [onVerify, onError])

  if (!TURNSTILE_SITE_KEY) {
    return null
  }

  return <div ref={ref} data-testid="turnstile-widget" />
}
