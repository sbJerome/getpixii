// Thin client for the Rust backend. Persists all site data to Postgres.
// Identity is the logged-in email, passed via the X-Pixii-User header.
// Every call degrades gracefully: if the backend is unreachable the app
// keeps working from in-memory seed data.

const headers = (email) => ({
  "Content-Type": "application/json",
  "X-Pixii-User": email || "guest@getpixii.ai",
});

export const api = {
  async auth(mode, { email, password, name }) {
    try {
      const r = await fetch("/api/auth", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ mode, email, password, name }),
      });
      return r.ok ? await r.json() : null;
    } catch {
      return null;
    }
  },

  async loadState(email) {
    try {
      const r = await fetch("/api/state", { headers: headers(email) });
      if (r.status === 204 || !r.ok) return null;
      return await r.json();
    } catch {
      return null;
    }
  },

  async saveState(email, data) {
    try {
      await fetch("/api/state", {
        method: "PUT",
        headers: headers(email),
        body: JSON.stringify(data),
      });
      return true;
    } catch {
      return false;
    }
  },

  // Global reference/catalog data (plans, themes, institutions, landing stats).
  async getCatalog() {
    try {
      const r = await fetch("/api/catalog");
      if (!r.ok) return null;
      return await r.json();
    } catch {
      return null;
    }
  },

  async putCatalog(data) {
    try {
      await fetch("/api/catalog", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(data),
      });
      return true;
    } catch {
      return false;
    }
  },
};
