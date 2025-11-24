export default function Footer() {
  return (
    <footer className="site-footer">
      <div className="footer-columns">
        <div>
          <h3 className="footer-title">W9 Group</h3>
          <p className="footer-copy">
            Daily reminders orchestrated by Rust + Next.js. Emails generated with Cerebras zai-glm-4.6 and delivered through W9 Mail
            infrastructure.
          </p>
        </div>
        <div>
          <h3 className="footer-title">Links</h3>
          <ul className="footer-links">
            <li>
              <a href="https://w9.nu" target="_blank" rel="noreferrer">
                W9 Mail
              </a>
            </li>
            <li>
              <a href="https://w9.se" target="_blank" rel="noreferrer">
                W9 Tools
              </a>
            </li>
          </ul>
        </div>
      </div>
    </footer>
  )
}
