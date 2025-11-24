import type { Metadata, Viewport } from 'next'
import Script from 'next/script'
import './globals.css'
import Header from './components/Header'
import Footer from './components/Footer'

export const metadata: Metadata = {
  title: 'W9 Daily Reminders',
  description: 'AI-assisted daily reminders synced with Google Calendar',
  manifest: '/site.webmanifest',
  icons: {
    icon: [
      { url: '/favicon.ico', sizes: 'any' },
      { url: '/favicon-32x32.png', sizes: '32x32', type: 'image/png' },
      { url: '/favicon-16x16.png', sizes: '16x16', type: 'image/png' },
    ],
    apple: [{ url: '/apple-touch-icon.png', sizes: '180x180', type: 'image/png' }],
  },
}

export const viewport: Viewport = {
  themeColor: '#000000',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <body>
        <Script src="https://challenges.cloudflare.com/turnstile/v0/api.js" strategy="afterInteractive" async defer />
        <div className="page-container">
          <Header />
          <main className="app">{children}</main>
          <Footer />
        </div>
      </body>
    </html>
  )
}
