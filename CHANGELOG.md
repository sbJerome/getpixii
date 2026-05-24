# Changelog

All notable changes to getpixii.ai are documented here.

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
