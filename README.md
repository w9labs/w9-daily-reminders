# W9 Daily Reminders

**AI-powered daily briefing system that delivers personalized schedule reminders with intelligent content generation and visual storytelling.**

W9 Daily Reminders is a production-grade orchestration service that synchronizes Google Calendar events, generates AI-crafted email content, and delivers beautifully formatted daily briefings at user-specified times. Built with enterprise-ready architecture, it integrates seamlessly with the W9 Mail infrastructure for authentication, user management, and transactional email delivery.

## Overview

W9 Daily Reminders transforms calendar data into actionable daily briefings through a sophisticated pipeline:

- **Calendar Synchronization**: Secure OAuth2 integration with Google Calendar for real-time event fetching
- **AI Content Generation**: Leverages Cerebras zai-glm-4.6 for intelligent, context-aware email copy generation
- **Visual Storytelling**: Pollinations.ai integration for dynamic image generation with customizable models
- **Weather Intelligence**: Location-based weather advisories with actionable recommendations
- **Multi-language Support**: Flexible language configuration with custom language support
- **Enterprise Email Delivery**: Integrated with W9 Mail for reliable, authenticated email sending

## Features

### Core Functionality
- **Automated Daily Reminders**: Scheduled email delivery at user-configured times
- **Google Calendar Integration**: OAuth2-based calendar sync with secure token management
- **AI-Generated Content**: Context-aware email generation using Cerebras zai-glm-4.6
- **Dynamic Image Generation**: Pollinations.ai integration with model selection and caching
- **Weather Advisories**: Location-based weather insights with practical recommendations
- **Timezone Support**: Full IANA timezone database support for global users

### User Management & Security
- **W9 Mail Integration**: Unified authentication and user management via W9 Mail API
- **Role-Based Access Control**: Admin, developer, and user privilege levels
- **Cloudflare Turnstile**: Bot protection on authentication endpoints
- **Email Verification**: Secure account verification workflow
- **Password Reset**: Self-service password recovery via email

### Administrative Features
- **System Configuration**: Runtime configuration management for mail API and senders
- **Sender Management**: Configurable daily reminder and no-reply sender selection
- **Test Email Delivery**: Preview and test email functionality for administrators
- **Health Monitoring**: Real-time system health and scheduler status

## Tech Stack

### Backend
- **Language**: Rust
- **Framework**: Axum (async web framework)
- **Storage**: JSON-based file storage with `DataStore`
- **APIs Integrated**:
  - Google Calendar API (OAuth2)
  - Cerebras API (zai-glm-4.6)
  - Pollinations.ai API (image generation)
  - W9 Mail API (authentication & email delivery)
  - Open-Meteo API (weather data)

### Frontend
- **Framework**: Next.js 14+ (App Router)
- **Language**: TypeScript
- **Styling**: TailwindCSS
- **Authentication**: JWT-based with W9 Mail integration
- **Security**: Cloudflare Turnstile integration

### Deployment
- **Containerization**: Docker with multi-stage builds
- **CI/CD**: GitHub Actions for automated builds
- **Image Registry**: GitHub Container Registry (GHCR)
- **Auto-updates**: Watchtower for zero-downtime updates
- **Reverse Proxy**: Nginx (configured separately)

## Quick Start

### Prerequisites
- Docker and Docker Compose installed on your VPS
- GitHub Container Registry (GHCR) access configured
- Google OAuth credentials (for Calendar integration)
- Cerebras API key (for AI content generation)

### Docker Deployment

This project uses **Docker with CI/CD** for deployment. Images are automatically built and pushed to GitHub Container Registry on every push to `main`.

1. **Set up deployment on your VPS:**
   
   Clone the repository and configure environment variables:
   
   ```bash
   # On your VPS
   git clone https://github.com/your-username/w9-daily-reminders.git
   cd w9-daily-reminders
   
   # Configure environment variables
   cp .env.example .env
   nano .env
   ```

2. **Login to GitHub Container Registry (if using private images):**
   ```bash
   echo $GITHUB_TOKEN | docker login ghcr.io -u YOUR_USERNAME --password-stdin
   ```

3. **Deploy with Docker Compose:**
   ```bash
   # Pull latest images
   docker-compose pull
   
   # Start services
   docker-compose up -d
   ```

4. **View logs:**
   ```bash
   docker-compose logs -f w9-daily-reminders-backend
   ```

### CI/CD

The project includes a GitHub Actions workflow (`.github/workflows/docker-build.yml`) that:
- Builds both backend and frontend Docker images on every push to `main`
- Pushes images to `ghcr.io/<your-username>/w9-daily-reminders-backend` and `w9-daily-reminders-frontend`
- Tags images with `latest` and commit SHA

**Watchtower** (included in docker-compose.yml) automatically updates containers when new images are pushed.
- Generate SSL certificates
- Configure environment variables from `.env`

### Configuration

Key environment variables (see `.env.example` for full list):

```bash
# API Keys
CEREBRAS_API_KEY=sk-your-cerebras-key
CEREBRAS_MODEL=zai-glm-4.6
POLLINATIONS_API_KEY=your-pollinations-token  # Optional, for premium models
POLLINATIONS_API_BASE=https://api.pollinations.ai

# Google OAuth
GOOGLE_CLIENT_ID=your-google-client-id.apps.googleusercontent.com
GOOGLE_CLIENT_SECRET=your-google-client-secret
GOOGLE_REDIRECT_URI=https://reminder.w9.nu/google/callback

# W9 Mail Integration
W9_MAIL_API_BASE=https://w9.nu/api
W9_MAIL_SERVICE_TOKEN=your-service-token

# Cloudflare Turnstile
TURNSTILE_SITE_KEY=your-site-key
TURNSTILE_SECRET=your-secret-key

# Domain & Ports
DOMAIN=reminder.w9.nu
PORT=8787
```

## Architecture

### System Components

1. **Frontend (Next.js)**
   - Static site generation with client-side interactivity
   - Direct API calls to W9 Mail for authentication
   - Local backend API for reminder management
   - Admin dashboard for system configuration

2. **Backend (Axum)**
   - RESTful API for reminder settings and previews
   - Google Calendar OAuth2 flow handling
   - AI content generation orchestration
   - Image generation with model caching
   - System configuration management

3. **Data Persistence**
   - JSON-based file storage in `DATA_DIR`
   - Settings, tokens, and configuration stored locally
   - Secure token encryption for Google OAuth

4. **Email Delivery**
   - Integration with W9 Mail API
   - Configurable sender selection (daily/noreply)
   - HTML email templates with W9 branding
   - Test email functionality

### API Endpoints

#### Public Endpoints
- `GET /api/settings` - Retrieve reminder settings
- `POST /api/settings` - Update reminder settings
- `POST /api/reminders/preview` - Generate email preview
- `GET /api/system/health` - System health status
- `GET /api/system/image-models` - Available image models

#### Admin Endpoints (Requires JWT)
- `GET /api/system/config` - Get system configuration
- `POST /api/system/config` - Update system configuration
- `GET /api/system/senders` - List available email senders
- `POST /api/reminders/send-test` - Send test email

#### Google OAuth
- `POST /api/google/start` - Initiate OAuth flow
- `POST /api/google/callback` - Handle OAuth callback

## Usage

### Setting Up Daily Reminders

1. **Configure Settings**:
   - Set your email address for delivery
   - Choose reminder time and timezone
   - Select language preference
   - Configure weather location
   - Enable/disable weather and image inclusion
   - Select image generation model (optional)

2. **Connect Google Calendar**:
   - Click "Sync Google Calendar" in settings
   - Authorize access in the OAuth popup
   - Calendar events will be automatically fetched

3. **Preview & Test**:
   - Use the Preview page to see generated emails
   - Test email delivery (admin only)
   - Adjust settings as needed

### Admin Functions

Administrators can:
- Configure W9 Mail API base URL and service tokens
- Select default senders for daily reminders and no-reply emails
- Manage system configuration
- Send test emails to verify delivery

## Development

### Local Development

```bash
# Backend
cd backend
cargo run

# Frontend
cd frontend
npm install
npm run dev
```

### Building

```bash
# Backend
cd backend
cargo build --release

# Frontend
cd frontend
npm run build
```

### Service Management

```bash
# Start service
sudo systemctl start w9-daily-reminders

# Stop service
sudo systemctl stop w9-daily-reminders

# View logs
sudo journalctl -u w9-daily-reminders -f

# Restart service
sudo systemctl restart w9-daily-reminders
```

## Security

- **Authentication**: JWT-based authentication via W9 Mail
- **Authorization**: Role-based access control (admin/dev/user)
- **Bot Protection**: Cloudflare Turnstile on sensitive endpoints
- **Token Security**: OAuth tokens encrypted and stored securely
- **API Security**: Service tokens never exposed in API responses
- **HTTPS**: SSL/TLS encryption for all communications

## Troubleshooting

### Common Issues

**Google Calendar not syncing**:
- Verify OAuth credentials in `.env`
- Check redirect URI matches Google Cloud Console
- Review backend logs: `journalctl -u w9-daily-reminders`

**Email delivery failing**:
- Verify W9 Mail API base URL and service token
- Check sender configuration in system settings
- Ensure W9 Mail service is accessible

**Image generation errors**:
- Check Pollinations API key (if using premium models)
- Verify model availability via `/api/system/image-models`
- Review API rate limits

**Preview not generating**:
- Verify Cerebras API key and model configuration
- Check backend logs for API errors
- Ensure settings are saved before preview

## License

GNU v3.0 License - See LICENSE file for details

## Support

For issues, questions, or contributions:
- Visit [W9 Mail](https://w9.nu) for infrastructure support
- Check system health at `/system` endpoint
- Review logs: `journalctl -u w9-daily-reminders`

---

**Built with ❤️ by the W9 Labs**
