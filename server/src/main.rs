use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use std::net::SocketAddr;
use tower_http::services::{ServeDir, ServeFile};

#[derive(Clone)]
struct App {
    db: PgPool,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .init();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "./dist".into());
    let bind = std::env::var("BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());

    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&db_url)
        .await
        .expect("failed to connect to Postgres");

    sqlx::raw_sql(include_str!("../migrations/0001_init.sql"))
        .execute(&db)
        .await
        .expect("migration failed");
    tracing::info!("migrations applied");

    let app_state = App { db };

    let index = format!("{static_dir}/index.html");
    let spa = ServeDir::new(&static_dir).not_found_service(ServeFile::new(index));

    let api = Router::new()
        .route("/health", get(health))
        .route("/auth", post(auth))
        .route("/state", get(get_state).put(put_state))
        .with_state(app_state);

    let app = Router::new().nest("/api", api).fallback_service(spa);

    let addr: SocketAddr = bind.parse().expect("bad BIND addr");
    tracing::info!("getpixii server listening on {addr}, serving {static_dir}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> impl IntoResponse {
    Json(json!({"status":"ok","service":"getpixii"}))
}

fn hash_pw(email: &str, pw: &str) -> String {
    let mut h = Sha256::new();
    h.update(email.as_bytes());
    h.update(b":");
    h.update(pw.as_bytes());
    hex::encode(h.finalize())
}

fn user_header(headers: &HeaderMap) -> String {
    headers
        .get("x-pixii-user")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "guest@getpixii.ai".into())
}

async fn ensure_user(db: &PgPool, email: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO users(email) VALUES($1) ON CONFLICT(email) DO NOTHING")
        .bind(email)
        .execute(db)
        .await?;
    Ok(())
}

// POST /api/auth  { mode, email, password, name }
async fn auth(State(app): State<App>, Json(body): Json<Value>) -> impl IntoResponse {
    let mode = body.get("mode").and_then(|v| v.as_str()).unwrap_or("login");
    let email = body
        .get("email")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_lowercase();
    let password = body.get("password").and_then(|v| v.as_str()).unwrap_or("");
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("").trim();

    if email.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({"error":"email required"}))).into_response();
    }
    let ph = hash_pw(&email, password);

    let existing: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT name, password_hash FROM users WHERE email=$1")
            .bind(&email)
            .fetch_optional(&app.db)
            .await
            .unwrap_or(None);

    let (final_name, ok) = match (mode, existing) {
        ("register", Some((cur_name, _))) => {
            // account exists; update name + password if provided
            let nm = if !name.is_empty() { name.to_string() } else { cur_name };
            let _ = sqlx::query("UPDATE users SET name=$2, password_hash=$3, updated_at=now() WHERE email=$1")
                .bind(&email).bind(&nm).bind(&ph)
                .execute(&app.db).await;
            (nm, true)
        }
        ("register", None) => {
            let nm = if !name.is_empty() { name.to_string() } else { email.clone() };
            let _ = sqlx::query("INSERT INTO users(email,name,password_hash) VALUES($1,$2,$3)")
                .bind(&email).bind(&nm).bind(&ph)
                .execute(&app.db).await;
            (nm, true)
        }
        (_, Some((cur_name, stored))) => {
            // login: verify if a password is set; lenient first-login binding otherwise
            match stored {
                Some(s) if !s.is_empty() => {
                    if s == ph { (cur_name, true) } else { (cur_name, false) }
                }
                _ => {
                    let _ = sqlx::query("UPDATE users SET password_hash=$2, updated_at=now() WHERE email=$1")
                        .bind(&email).bind(&ph).execute(&app.db).await;
                    (cur_name, true)
                }
            }
        }
        (_, None) => {
            // login for unknown account → create it (demo-friendly)
            let nm = if !name.is_empty() { name.to_string() } else { email.clone() };
            let _ = sqlx::query("INSERT INTO users(email,name,password_hash) VALUES($1,$2,$3)")
                .bind(&email).bind(&nm).bind(&ph)
                .execute(&app.db).await;
            (nm, true)
        }
    };

    if !ok {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error":"invalid credentials"}))).into_response();
    }
    Json(json!({"token": ph, "user": {"name": final_name, "email": email}})).into_response()
}

// GET /api/state
async fn get_state(State(app): State<App>, headers: HeaderMap) -> impl IntoResponse {
    let email = user_header(&headers);

    let urow = sqlx::query("SELECT name, plan, theme_id, prefs, saved_scenarios, budgets FROM users WHERE email=$1")
        .bind(&email)
        .fetch_optional(&app.db)
        .await
        .unwrap_or(None);

    let Some(urow) = urow else {
        return StatusCode::NO_CONTENT.into_response();
    };

    let name: String = urow.get("name");
    let plan: String = urow.get("plan");
    let theme_id: String = urow.get("theme_id");
    let prefs: Value = urow.get("prefs");
    let saved_scenarios: Value = urow.get("saved_scenarios");
    let budgets: Value = urow.get("budgets");

    let accounts = sqlx::query(
        "SELECT id,name,inst,type,balance,limit_amt,apr,reconciled,last_seen FROM accounts WHERE user_email=$1 ORDER BY pos",
    )
    .bind(&email)
    .fetch_all(&app.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| {
        json!({
            "id": r.get::<String,_>("id"),
            "name": r.get::<String,_>("name"),
            "inst": r.get::<Option<String>,_>("inst"),
            "type": r.get::<String,_>("type"),
            "balance": r.get::<f64,_>("balance"),
            "limit": r.get::<Option<f64>,_>("limit_amt"),
            "apr": r.get::<Option<f64>,_>("apr"),
            "reconciled": r.get::<bool,_>("reconciled"),
            "last": r.get::<Option<String>,_>("last_seen"),
        })
    })
    .collect::<Vec<_>>();

    let tx = sqlx::query(
        "SELECT id,tx_date,merchant,category,amount,account,conf FROM transactions WHERE user_email=$1 ORDER BY pos",
    )
    .bind(&email)
    .fetch_all(&app.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| {
        json!({
            "id": r.get::<String,_>("id"),
            "date": r.get::<Option<String>,_>("tx_date"),
            "merchant": r.get::<Option<String>,_>("merchant"),
            "category": r.get::<Option<String>,_>("category"),
            "amount": r.get::<f64,_>("amount"),
            "account": r.get::<Option<String>,_>("account"),
            "conf": r.get::<Option<f64>,_>("conf"),
        })
    })
    .collect::<Vec<_>>();

    let goals = sqlx::query("SELECT id,name,target,saved FROM goals WHERE user_email=$1 ORDER BY pos")
        .bind(&email)
        .fetch_all(&app.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| {
            json!({
                "id": r.get::<String,_>("id"),
                "name": r.get::<String,_>("name"),
                "target": r.get::<f64,_>("target"),
                "saved": r.get::<f64,_>("saved"),
            })
        })
        .collect::<Vec<_>>();

    let agents = sqlx::query("SELECT id,name,enabled,descr,stat,impact FROM agents WHERE user_email=$1 ORDER BY pos")
        .bind(&email)
        .fetch_all(&app.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| {
            json!({
                "id": r.get::<String,_>("id"),
                "name": r.get::<String,_>("name"),
                "on": r.get::<bool,_>("enabled"),
                "desc": r.get::<Option<String>,_>("descr"),
                "stat": r.get::<Option<String>,_>("stat"),
                "impact": r.get::<f64,_>("impact"),
            })
        })
        .collect::<Vec<_>>();

    Json(json!({
        "name": name,
        "plan": plan,
        "themeId": theme_id,
        "prefs": prefs,
        "savedScenarios": saved_scenarios,
        "budgets": budgets,
        "accounts": accounts,
        "tx": tx,
        "goals": goals,
        "agents": agents,
    }))
    .into_response()
}

fn f64_of(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0)
}
fn opt_f64(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(|x| x.as_f64())
}
fn str_of<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("")
}
fn opt_str<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(|x| x.as_str())
}

// PUT /api/state  — replaces the user's entire dataset
async fn put_state(
    State(app): State<App>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let email = user_header(&headers);
    if let Err(e) = ensure_user(&app.db, &email).await {
        tracing::error!("ensure_user: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let name = str_of(&body, "name");
    let plan = body.get("plan").and_then(|v| v.as_str()).unwrap_or("plus");
    let theme_id = body.get("themeId").and_then(|v| v.as_str()).unwrap_or("light");
    let prefs = body.get("prefs").cloned().unwrap_or_else(|| json!({}));
    let saved = body.get("savedScenarios").cloned().unwrap_or_else(|| json!([]));
    let budgets = body.get("budgets").cloned().unwrap_or_else(|| json!({}));

    let mut txn = match app.db.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("begin: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let res: Result<(), sqlx::Error> = async {
        sqlx::query(
            "UPDATE users SET name=$2, plan=$3, theme_id=$4, prefs=$5, saved_scenarios=$6, budgets=$7, updated_at=now() WHERE email=$1",
        )
        .bind(&email)
        .bind(if name.is_empty() { &email } else { name })
        .bind(plan)
        .bind(theme_id)
        .bind(&prefs)
        .bind(&saved)
        .bind(&budgets)
        .execute(&mut *txn)
        .await?;

        sqlx::query("DELETE FROM accounts WHERE user_email=$1").bind(&email).execute(&mut *txn).await?;
        sqlx::query("DELETE FROM transactions WHERE user_email=$1").bind(&email).execute(&mut *txn).await?;
        sqlx::query("DELETE FROM goals WHERE user_email=$1").bind(&email).execute(&mut *txn).await?;
        sqlx::query("DELETE FROM agents WHERE user_email=$1").bind(&email).execute(&mut *txn).await?;

        if let Some(arr) = body.get("accounts").and_then(|v| v.as_array()) {
            for (i, a) in arr.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO accounts(user_email,id,name,inst,type,balance,limit_amt,apr,reconciled,last_seen,pos) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
                )
                .bind(&email)
                .bind(str_of(a, "id"))
                .bind(str_of(a, "name"))
                .bind(opt_str(a, "inst"))
                .bind(if str_of(a, "type").is_empty() { "checking" } else { str_of(a, "type") })
                .bind(f64_of(a, "balance"))
                .bind(opt_f64(a, "limit"))
                .bind(opt_f64(a, "apr"))
                .bind(a.get("reconciled").and_then(|x| x.as_bool()).unwrap_or(false))
                .bind(opt_str(a, "last"))
                .bind(i as i32)
                .execute(&mut *txn)
                .await?;
            }
        }

        if let Some(arr) = body.get("tx").and_then(|v| v.as_array()) {
            for (i, t) in arr.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO transactions(user_email,id,tx_date,merchant,category,amount,account,conf,pos) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)",
                )
                .bind(&email)
                .bind(str_of(t, "id"))
                .bind(opt_str(t, "date"))
                .bind(opt_str(t, "merchant"))
                .bind(opt_str(t, "category"))
                .bind(f64_of(t, "amount"))
                .bind(opt_str(t, "account"))
                .bind(opt_f64(t, "conf"))
                .bind(i as i32)
                .execute(&mut *txn)
                .await?;
            }
        }

        if let Some(arr) = body.get("goals").and_then(|v| v.as_array()) {
            for (i, g) in arr.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO goals(user_email,id,name,target,saved,pos) VALUES($1,$2,$3,$4,$5,$6)",
                )
                .bind(&email)
                .bind(str_of(g, "id"))
                .bind(str_of(g, "name"))
                .bind(f64_of(g, "target"))
                .bind(f64_of(g, "saved"))
                .bind(i as i32)
                .execute(&mut *txn)
                .await?;
            }
        }

        if let Some(arr) = body.get("agents").and_then(|v| v.as_array()) {
            for (i, ag) in arr.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO agents(user_email,id,name,enabled,descr,stat,impact,pos) VALUES($1,$2,$3,$4,$5,$6,$7,$8)",
                )
                .bind(&email)
                .bind(str_of(ag, "id"))
                .bind(str_of(ag, "name"))
                .bind(ag.get("on").and_then(|x| x.as_bool()).unwrap_or(false))
                .bind(opt_str(ag, "desc"))
                .bind(opt_str(ag, "stat"))
                .bind(f64_of(ag, "impact"))
                .bind(i as i32)
                .execute(&mut *txn)
                .await?;
            }
        }

        Ok(())
    }
    .await;

    match res {
        Ok(_) => match txn.commit().await {
            Ok(_) => Json(json!({"ok":true})).into_response(),
            Err(e) => {
                tracing::error!("commit: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
        Err(e) => {
            tracing::error!("put_state txn: {e}");
            let _ = txn.rollback().await;
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
