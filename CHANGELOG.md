# Changelog

All notable changes to getpixii.ai are documented here.

## [1.2.3] - 2026-05-24

### Added
- SEO-friendly URLs + refresh-safe SPA routing (History API): `/`, `/login`,
  `/register`, `/app`, `/app/<workspace>`. Per-route titles, canonical/OG tags,
  `robots.txt`, `sitemap.xml`.
- Configurable AI base URL (`AI_BASE_URL`) — copilot runs on the in-cluster
  Meridian Claude proxy (claude-sonnet-4-6), no real key required.
- **Live data online**: imported API keys into `getpixii-secrets` — markets
  (Finnhub), news (NewsAPI), AI (proxy) and Plaid (sandbox) all verified live.

### Fixed
- SPA deep links return HTTP 200 (was 404); unknown `/api/*` returns 404 JSON.
- Empty-data crashes (Sankey `tx` scope, undefined `intel.card`, AddModal
  account default, NaN KPIs) — app now shows a clean "connect an account" state.
- Plaid `link_token` no longer sends the email as `client_user_id` (Plaid
  forbids PII); uses a stable SHA-256 hash instead.

## [1.1.0] - 2026-05-24

### Changed
- Updated to the expanded Pixii template (FinIntel, Goals, Bills, Credit,
  Reports, Markets, Wellness, Tools/calculators, Admin) — ~2950 lines.
- Removed the white page border (global `html/body/#root` reset).

### Added
- **Live-data connectors** (`server/src/live.rs`) reading API keys from env:
  - `GET /api/live/markets`, `GET /api/live/quotes` — Finnhub quotes
  - `GET /api/live/news` — NewsAPI (Marketaux fallback)
  - `POST /api/ai/chat` — Anthropic (OpenRouter fallback)
  - `POST /api/plaid/link_token`, `/api/plaid/exchange`, `GET /api/plaid/accounts`
- `getpixii-secrets` secret (envFrom, optional) for the live-data keys.

### Removed
- **All dummy/seed data.** Accounts, transactions, holdings, goals, budgets,
  history, invoices, institution balances, landing stats, and the illustrative
  arrays in Bills/Credit/Markets/Wellness/Admin are now empty by default. Data
  comes from Postgres (per-user) and the live connectors; the Markets index
  band and news pull live when keys are present.

## [1.0.2] - 2026-05-24

### Added
- All remaining visible data now lives in Postgres. Per-user `holdings`
  (investments), `invoices`, and `monthHistory` are stored on the users row
  (JSONB) and round-trip through `/api/state`.
- New `catalog` table + `GET/PUT /api/catalog` for global reference data
  (pricing plans, theme presets, institution catalog, landing stat band).
  The frontend seeds the catalog from its own constants on first load, then
  reads it back from the database (DB is the source of truth).

## [1.0.1] - 2026-05-24

### Fixed
- Startup migration now runs as a multi-statement batch (`sqlx::raw_sql`),
  fixing a crash-loop on first boot ("cannot insert multiple commands into a
  prepared statement").

## [1.0.0] - 2026-05-24

### Added
- Initial release of the Pixii finance OS, served by a Rust (axum) web server.
- React/Vite SPA built from the `pixii_v6.jsx` template, rendered verbatim.
- Rust + sqlx backend persisting all site data to the cluster PostgreSQL
  (`getpixii` database): users, accounts, transactions, goals, agents, plus
  prefs / saved scenarios / budgets as JSONB.
- REST API: `/api/health`, `/api/auth`, `GET/PUT /api/state`.
- Frontend data layer (`web/src/api.js`) hydrating state on login and
  autosaving every change (debounced), with graceful offline fallback.
- Kubernetes manifests: namespace, deployment, LoadBalancer Service on
  `10.50.0.52`.
- Multi-stage container image (`ghcr.io/sbjerome/getpixii`).
- Technitium DNS A record `getpixii.ai → 10.50.0.52`.
