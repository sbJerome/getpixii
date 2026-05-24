# getpixii.ai

**Pixii — the autonomous finance OS.** A single-page finance cockpit (forecasting,
investments, books, AI agents, copilot) served by a **Rust** web server that also
persists all site data to **PostgreSQL** running in the K3s cluster.

## Architecture

```
Browser ──HTTP──▶ Rust (axum) server  ──┬──▶ serves built React SPA (/, static)
        10.50.0.52                       └──▶ /api/*  ──▶ PostgreSQL (getpixii db)
```

- **Frontend** (`web/`): React + Vite + Recharts + lucide-react. The UI is the
  Pixii template (`pixii_v6.jsx`) rendered verbatim — no template changes. A thin
  data layer (`web/src/api.js`) hydrates state from the backend on login and
  autosaves every change.
- **Backend** (`server/`): Rust + axum + sqlx. Serves the static SPA with
  SPA-fallback routing and exposes the persistence API. Migrations run at startup.
- **Database**: `getpixii` database on `postgres.services.svc.cluster.local:5432`
  (shared cluster Postgres). Normalized tables: `users`, `accounts`,
  `transactions`, `goals`, `agents` plus JSONB columns for prefs / saved scenarios
  / budgets.

## API

| Method | Path          | Purpose                                            |
|--------|---------------|----------------------------------------------------|
| GET    | `/api/health` | Liveness/readiness probe                           |
| POST   | `/api/auth`   | Register / sign in (`{mode,email,password,name}`)  |
| GET    | `/api/state`  | Load the signed-in user's full dataset (204 if new)|
| PUT    | `/api/state`  | Replace the user's full dataset (debounced autosave)|

The signed-in identity is sent in the `X-Pixii-User` header.

## Local development

```bash
# backend (needs DATABASE_URL)
cd server && DATABASE_URL=postgres://... cargo run
# frontend (proxies /api to :8080)
cd web && npm install && npm run dev
```

## Build & deploy

```bash
podman build -t ghcr.io/sbjerome/getpixii:latest .
podman push ghcr.io/sbjerome/getpixii:latest
kubectl apply -f k8s/        # namespace, deployment, service
```

Live at **http://10.50.0.52** / **http://getpixii.ai** (Technitium DNS A record).
