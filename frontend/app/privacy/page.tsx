export default function PrivacyPage() {
  return (
    <>
      <div className="box">
        <h2 className="section-title">Data Collection</h2>
        <p>
          W9 Daily Reminders collects and stores the following information:
        </p>
        <ul className="list">
          <li>Email address for reminder delivery</li>
          <li>Google Calendar OAuth tokens (stored locally on the server)</li>
          <li>Reminder preferences (time, timezone, language, weather location)</li>
          <li>System configuration (mail API settings, sender selection)</li>
        </ul>
      </div>
      <div className="box">
        <h2 className="section-title">Data Usage</h2>
        <p>
          Your data is used exclusively to:
        </p>
        <ul className="list">
          <li>Generate and send daily reminder emails</li>
          <li>Sync with your Google Calendar to fetch upcoming events</li>
          <li>Fetch weather information for your specified location</li>
          <li>Generate email content using Cerebras AI and images using Pollinations</li>
        </ul>
      </div>
      <div className="box">
        <h2 className="section-title">Data Storage</h2>
        <p>
          All data is stored locally on the server where the application is deployed. 
          Google OAuth tokens are encrypted and stored securely. No data is shared with 
          third parties except:
        </p>
        <ul className="list">
          <li>Google Calendar API (for event fetching)</li>
          <li>Cerebras API (for email content generation)</li>
          <li>Pollinations API (for image generation)</li>
          <li>W9 Mail API (for email delivery)</li>
        </ul>
      </div>
      <div className="box">
        <h2 className="section-title">Data Deletion</h2>
        <p>
          You can delete your account and all associated data at any time through the 
          admin interface. Google OAuth tokens can be revoked through your Google account 
          settings. All locally stored data will be permanently deleted upon account removal.
        </p>
      </div>
      <div className="box">
        <h2 className="section-title">Contact</h2>
        <p>
          For questions about this privacy policy, contact the system administrator 
          or visit <a href="https://w9.nu" target="_blank" rel="noreferrer">W9 Mail</a>.
        </p>
      </div>
    </>
  )
}

