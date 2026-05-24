# Release Log

## v1.0.0 — 2026-05-24

- **Service:** getpixii.ai
- **Image:** `ghcr.io/sbjerome/getpixii:v1.0.0`
- **Cluster IP:** `10.50.0.52` (MetalLB LoadBalancer, next free in the 10.50.0.0 pool)
- **DNS:** Technitium A record `getpixii.ai → 10.50.0.52`
- **Database:** `getpixii` on `postgres.services.svc.cluster.local:5432`
- **Repos:** GitHub `sbJerome/getpixii`, Forgejo `sbjerome/getpixii`

First production deployment. Rust server serving the Pixii SPA with full
PostgreSQL persistence of all site data.
