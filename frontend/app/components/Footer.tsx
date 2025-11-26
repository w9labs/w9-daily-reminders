import Link from 'next/link'

export default function Footer() {
  return (
    <footer className="site-footer">
      <div className="footer-columns">
        <div>
          <h3 className="footer-title">Developed by W9 Labs</h3>
          <p className="footer-copy">
            Reminder engine built with Rust + Next.js + Cerebras + Cloudflare Workers AI. Email delivery powered by W9 Mail. Reach us at{' '}
            <a href="mailto:hi@w9.se">hi@w9.se</a>.
          </p>
        </div>
        <div>
          <h3 className="footer-title">Network</h3>
          <ul className="footer-links">
            <li>
              <a href="https://w9.se" target="_blank" rel="noreferrer">
                W9 Tools · Links & drops
              </a>
            </li>
            <li>
              <a href="https://w9.nu" target="_blank" rel="noreferrer">
                W9 Mail · Transactional rail
              </a>
            </li>
            <li>
              <a href="https://reminder.w9.nu" target="_blank" rel="noreferrer">
                W9 Daily Reminders · Calendar digest
              </a>
            </li>
          </ul>
        </div>
        <div>
          <h3 className="footer-title">Legal</h3>
          <ul className="footer-links">
            <li>
              <Link href="/terms">Terms of Service</Link>
            </li>
            <li>
              <Link href="/privacy">Privacy Notice</Link>
            </li>
          </ul>
        </div>
      </div>
      <div className="footer-bottom">
        © {new Date().getFullYear()} W9 Labs · All projects are open-source and community audited.
      </div>
    </footer>
  )
}
