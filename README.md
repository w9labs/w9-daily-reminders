# W9 Daily Reminders

AI-powered daily calendar digest service for W9 Labs.

## Tech Stack

- **Backend**: Rust + Axum + SurrealDB
- **Frontend**: Leptos (Full-stack SSR)
- **Integration**: Google Calendar API + W9 Mail API

## Features

- Google Calendar OAuth integration
- AI-powered event summarization
- Scheduled daily email digests via W9 Mail
- Customizable delivery times
- Calendar sync and management

## Quick Start

```bash
cargo run --package w9-daily-reminders-server
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | SurrealDB connection | `memory` |
| `W9_MAIL_API_URL` | W9 Mail API URL | `https://mail.w9.nu` |
| `W9_MAIL_API_TOKEN` | W9 Mail API token | (required) |
| `GOOGLE_CLIENT_ID` | Google OAuth client ID | (required) |
| `GOOGLE_CLIENT_SECRET` | Google OAuth secret | (required) |
| `PORT` | Server port | `8084` |

## Deployment

```bash
docker-compose up -d
```

Access at: `https://reminder.w9.nu`

## License

GPL v3.0
