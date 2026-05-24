CREATE TABLE IF NOT EXISTS users (
    email           TEXT PRIMARY KEY,
    name            TEXT NOT NULL DEFAULT '',
    password_hash   TEXT,
    plan            TEXT NOT NULL DEFAULT 'plus',
    theme_id        TEXT NOT NULL DEFAULT 'light',
    prefs           JSONB NOT NULL DEFAULT '{}'::jsonb,
    saved_scenarios JSONB NOT NULL DEFAULT '[]'::jsonb,
    budgets         JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS accounts (
    user_email  TEXT NOT NULL REFERENCES users(email) ON DELETE CASCADE,
    id          TEXT NOT NULL,
    name        TEXT NOT NULL DEFAULT '',
    inst        TEXT,
    type        TEXT NOT NULL DEFAULT 'checking',
    balance     DOUBLE PRECISION NOT NULL DEFAULT 0,
    limit_amt   DOUBLE PRECISION,
    apr         DOUBLE PRECISION,
    reconciled  BOOLEAN NOT NULL DEFAULT false,
    last_seen   TEXT,
    pos         INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_email, id)
);

CREATE TABLE IF NOT EXISTS transactions (
    user_email  TEXT NOT NULL REFERENCES users(email) ON DELETE CASCADE,
    id          TEXT NOT NULL,
    tx_date     TEXT,
    merchant    TEXT,
    category    TEXT,
    amount      DOUBLE PRECISION NOT NULL DEFAULT 0,
    account     TEXT,
    conf        DOUBLE PRECISION,
    pos         INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_email, id)
);

CREATE TABLE IF NOT EXISTS goals (
    user_email  TEXT NOT NULL REFERENCES users(email) ON DELETE CASCADE,
    id          TEXT NOT NULL,
    name        TEXT NOT NULL DEFAULT '',
    target      DOUBLE PRECISION NOT NULL DEFAULT 0,
    saved       DOUBLE PRECISION NOT NULL DEFAULT 0,
    pos         INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_email, id)
);

CREATE TABLE IF NOT EXISTS agents (
    user_email  TEXT NOT NULL REFERENCES users(email) ON DELETE CASCADE,
    id          TEXT NOT NULL,
    name        TEXT NOT NULL DEFAULT '',
    enabled     BOOLEAN NOT NULL DEFAULT false,
    descr       TEXT,
    stat        TEXT,
    impact      DOUBLE PRECISION NOT NULL DEFAULT 0,
    pos         INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_email, id)
);
