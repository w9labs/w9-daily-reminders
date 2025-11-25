export default function TermsPage() {
  return (
    <>
      <div className="box">
        <h2 className="section-title">Terms of Service</h2>
        <p>
          By using W9 Daily Reminders, you agree to the following terms and conditions.
        </p>
      </div>
      <div className="box">
        <h2 className="section-title">Service Description</h2>
        <p>
          W9 Daily Reminders is an AI-assisted daily reminder service that:
        </p>
        <ul className="list">
          <li>Syncs with your Google Calendar to fetch upcoming events</li>
          <li>Generates personalized reminder emails using AI (Cerebras zai-glm-4.6)</li>
          <li>Includes weather information for your specified location</li>
          <li>Generates visual content using Pollinations AI</li>
          <li>Delivers emails at your specified time via W9 Mail infrastructure</li>
        </ul>
      </div>
      <div className="box">
        <h2 className="section-title">User Responsibilities</h2>
        <p>
          You are responsible for:
        </p>
        <ul className="list">
          <li>Maintaining the security of your account credentials</li>
          <li>Ensuring your Google Calendar OAuth tokens remain valid</li>
          <li>Providing accurate email addresses and configuration settings</li>
          <li>Complying with all applicable laws and regulations</li>
          <li>Not using the service for spam, harassment, or illegal activities</li>
        </ul>
      </div>
      <div className="box">
        <h2 className="section-title">Service Availability</h2>
        <p>
          W9 Daily Reminders is provided "as is" without warranties of any kind. We do not guarantee:
        </p>
        <ul className="list">
          <li>Uninterrupted or error-free service</li>
          <li>Accuracy of AI-generated content</li>
          <li>Timely delivery of reminder emails</li>
          <li>Availability of third-party services (Google Calendar, Cerebras, Pollinations)</li>
        </ul>
      </div>
      <div className="box">
        <h2 className="section-title">Data and Privacy</h2>
        <p>
          Your use of this service is subject to our <a href="/privacy">Privacy Policy</a>. 
          We store your configuration, Google OAuth tokens, and email preferences locally on the server. 
          Data is not shared with third parties except as necessary to provide the service (Google Calendar API, 
          Cerebras API, Pollinations API, W9 Mail API).
        </p>
      </div>
      <div className="box">
        <h2 className="section-title">Limitation of Liability</h2>
        <p>
          W9 Daily Reminders and its operators are not liable for:
        </p>
        <ul className="list">
          <li>Missed reminders or delayed email delivery</li>
          <li>Inaccuracies in AI-generated content</li>
          <li>Loss of data or service interruptions</li>
          <li>Issues arising from third-party service outages</li>
          <li>Any indirect, incidental, or consequential damages</li>
        </ul>
      </div>
      <div className="box">
        <h2 className="section-title">Termination</h2>
        <p>
          We reserve the right to suspend or terminate your access to the service at any time, 
          with or without notice, for violation of these terms or for any other reason. 
          You may delete your account and data at any time through the admin interface.
        </p>
      </div>
      <div className="box">
        <h2 className="section-title">Changes to Terms</h2>
        <p>
          We may update these terms of service at any time. Continued use of the service 
          after changes constitutes acceptance of the new terms. We recommend reviewing 
          this page periodically.
        </p>
      </div>
      <div className="box">
        <h2 className="section-title">Contact</h2>
        <p>
          For questions about these terms, contact the system administrator or visit 
          <a href="https://w9.nu" target="_blank" rel="noreferrer"> W9 Mail</a>.
        </p>
      </div>
    </>
  )
}

