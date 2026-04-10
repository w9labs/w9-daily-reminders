# W9 Daily Reminders · Mail + Auth Integration Plan

## Goals
- Reuse the W9 Mail API for authentication, user management, and transactional email sending.
- Introduce two configurable senders (daily reminder + no-reply) that map to W9 Mail accounts or aliases.
- Require Cloudflare Turnstile challenges on security-sensitive forms (login, register, password reset, verification).
- Enforce admin-only controls (sender selection, user management) by validating JWTs issued by W9 Mail.
- Keep service-to-service automation separate via a stored API token minted from W9 Mail's "API Tokens" feature.

## High-Level Architecture
1. **Frontend (Next.js)**
   - Mirrors W9 Mail / W9 Tools UX primitives (header/footer, nav, Courier aesthetic).
   - Provides pages: login, register, verify email, reset password, profile, admin dashboard.
   - Calls W9 Mail REST endpoints directly for auth (`/api/auth/*`) and user management.
   - Calls the local Reminders backend for reminder settings, previews, Pollinations outputs, and sender configuration.
   - Supplies `Authorization: Bearer <jwt>` (token issued by W9 Mail) to local admin endpoints; backend validates via `GET {MAIL_API}/auth/me`.

2. **Backend (Axum)**
   - Persists reminder settings + system config (mail API base URL, service API token, daily sender, no-reply sender) inside `DataStore`.
   - Provides new routes:
     - `GET/POST /api/system/config` (admin-only, handles sender + token configuration; never echoes secrets in responses).
     - `POST /api/reminders/send-test` (admin-only, uses service API token to call W9 Mail `/api/send`).
     - Existing `/api/reminders/preview` now appends header + footer content.
   - Adds `W9MailClient` module for:
     - `get_profile(token)` – validate JWT via `/auth/me`.
     - `list_senders(token)` – fan-out to `/accounts` + `/aliases` for UI pickers.
     - `send_email(service_token, request)` – invoke `/send` with stored token.

3. **Configuration Sources**
   - `.env` / `install.sh` define defaults: `W9_MAIL_API_BASE`, `TURNSTILE_*`, `SERVICE_TOKEN` fallback.
   - Admins can override runtime values through the UI; backend writes them to `data/config.json`.

4. **Security**
   - All privileged backend routes require `Authorization` header with JWT; backend rejects if `/auth/me` response is missing or not `admin`.
   - Service API token is never returned via API after being saved.
   - Cloudflare Turnstile component reused from W9 Mail to gate login/register/reset actions on the frontend.

## Next Steps
1. Implement backend `w9mail.rs`, `SystemConfig`, and guarded routes.
2. Extend frontend with layout header/footer, auth pages, Turnstile widget, and admin screens for sender config.
3. Wire email preview + eventual dispatch to Pollinations images + W9 Mail send endpoint.
