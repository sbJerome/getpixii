// Live-data connectors. Every handler reads its API key from the environment
// (populated from the .credentials file -> k8s secret). When a key is absent the
// endpoint returns an empty/clear result rather than fabricated data.

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::HashMap;

use crate::App;

fn env(k: &str) -> Option<String> {
    std::env::var(k).ok().filter(|s| !s.trim().is_empty())
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("getpixii/1.1")
        .build()
        .unwrap_or_default()
}

const DEFAULT_MARKET: &[(&str, &str)] = &[
    ("SPY", "S&P 500"),
    ("QQQ", "Nasdaq"),
    ("DIA", "Dow"),
    ("IWM", "Russell 2000"),
    ("BTC-USD", "Bitcoin"),
    ("GLD", "Gold"),
];

// GET /api/live/markets  — index proxies (or ?symbols=AAPL,MSFT)
pub async fn markets(Query(q): Query<HashMap<String, String>>) -> impl IntoResponse {
    let Some(key) = env("FINNHUB_KEY") else {
        return Json(json!({"connected": false, "items": []})).into_response();
    };
    let pairs: Vec<(String, String)> = match q.get("symbols") {
        Some(s) => s.split(',').map(|x| (x.trim().to_uppercase(), x.trim().to_uppercase())).collect(),
        None => DEFAULT_MARKET.iter().map(|(s, l)| (s.to_string(), l.to_string())).collect(),
    };
    let c = client();
    let mut items = Vec::new();
    for (sym, label) in pairs {
        let url = format!("https://finnhub.io/api/v1/quote?symbol={sym}&token={key}");
        if let Ok(resp) = c.get(&url).send().await {
            if let Ok(v) = resp.json::<Value>().await {
                let price = v.get("c").and_then(|x| x.as_f64()).unwrap_or(0.0);
                let dp = v.get("dp").and_then(|x| x.as_f64()).unwrap_or(0.0);
                if price > 0.0 {
                    items.push(json!({
                        "symbol": sym, "label": label, "price": price,
                        "changePct": dp, "up": dp >= 0.0
                    }));
                }
            }
        }
    }
    Json(json!({"connected": true, "items": items})).into_response()
}

// GET /api/live/quotes?symbols=AAPL,MSFT
pub async fn quotes(Query(q): Query<HashMap<String, String>>) -> impl IntoResponse {
    let Some(key) = env("FINNHUB_KEY") else {
        return Json(json!({"connected": false, "items": []})).into_response();
    };
    let syms: Vec<String> = q
        .get("symbols")
        .map(|s| s.split(',').map(|x| x.trim().to_uppercase()).filter(|x| !x.is_empty()).collect())
        .unwrap_or_default();
    let c = client();
    let mut items = Vec::new();
    for sym in syms {
        let url = format!("https://finnhub.io/api/v1/quote?symbol={sym}&token={key}");
        if let Ok(resp) = c.get(&url).send().await {
            if let Ok(v) = resp.json::<Value>().await {
                items.push(json!({
                    "symbol": sym,
                    "price": v.get("c").and_then(|x| x.as_f64()).unwrap_or(0.0),
                    "changePct": v.get("dp").and_then(|x| x.as_f64()).unwrap_or(0.0),
                    "high": v.get("h").and_then(|x| x.as_f64()),
                    "low": v.get("l").and_then(|x| x.as_f64()),
                }));
            }
        }
    }
    Json(json!({"connected": true, "items": items})).into_response()
}

// GET /api/live/news  — market/business headlines
pub async fn news() -> impl IntoResponse {
    let c = client();
    if let Some(key) = env("NEWSAPI_KEY") {
        let url = format!("https://newsapi.org/v2/top-headlines?category=business&language=en&pageSize=12&apiKey={key}");
        if let Ok(resp) = c.get(&url).send().await {
            if let Ok(v) = resp.json::<Value>().await {
                let items: Vec<Value> = v.get("articles").and_then(|a| a.as_array()).map(|arr| {
                    arr.iter().map(|a| json!({
                        "title": a.get("title").and_then(|x| x.as_str()).unwrap_or(""),
                        "source": a.get("source").and_then(|s| s.get("name")).and_then(|x| x.as_str()).unwrap_or(""),
                        "url": a.get("url").and_then(|x| x.as_str()).unwrap_or(""),
                        "publishedAt": a.get("publishedAt").and_then(|x| x.as_str()).unwrap_or(""),
                    })).collect()
                }).unwrap_or_default();
                return Json(json!({"connected": true, "items": items})).into_response();
            }
        }
    }
    if let Some(key) = env("MARKETAUX_KEY") {
        let url = format!("https://api.marketaux.com/v1/news/all?language=en&filter_entities=true&limit=12&api_token={key}");
        if let Ok(resp) = c.get(&url).send().await {
            if let Ok(v) = resp.json::<Value>().await {
                let items: Vec<Value> = v.get("data").and_then(|a| a.as_array()).map(|arr| {
                    arr.iter().map(|a| json!({
                        "title": a.get("title").and_then(|x| x.as_str()).unwrap_or(""),
                        "source": a.get("source").and_then(|x| x.as_str()).unwrap_or(""),
                        "url": a.get("url").and_then(|x| x.as_str()).unwrap_or(""),
                        "publishedAt": a.get("published_at").and_then(|x| x.as_str()).unwrap_or(""),
                    })).collect()
                }).unwrap_or_default();
                return Json(json!({"connected": true, "items": items})).into_response();
            }
        }
    }
    Json(json!({"connected": false, "items": []})).into_response()
}

// POST /api/ai/chat  { prompt }  — Anthropic, fallback OpenRouter
pub async fn ai_chat(Json(body): Json<Value>) -> impl IntoResponse {
    let prompt = body.get("prompt").and_then(|x| x.as_str()).unwrap_or("").to_string();
    if prompt.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "prompt required"}))).into_response();
    }
    let c = client();

    if let Some(key) = env("ANTHROPIC_API_KEY") {
        let model = env("AI_MODEL").unwrap_or_else(|| "claude-opus-4-7".into());
        let payload = json!({
            "model": model,
            "max_tokens": 1024,
            "messages": [{"role": "user", "content": prompt}]
        });
        if let Ok(resp) = c
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .json(&payload)
            .send()
            .await
        {
            if let Ok(v) = resp.json::<Value>().await {
                let reply = v.get("content")
                    .and_then(|c| c.as_array())
                    .and_then(|a| a.first())
                    .and_then(|b| b.get("text"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                if !reply.is_empty() {
                    return Json(json!({"reply": reply, "provider": "anthropic"})).into_response();
                }
            }
        }
    }

    if let Some(key) = env("OPENROUTER_API_KEY") {
        let model = env("AI_FALLBACK_MODEL").unwrap_or_else(|| "openai/gpt-4o".into());
        let payload = json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}]
        });
        if let Ok(resp) = c
            .post("https://openrouter.ai/api/v1/chat/completions")
            .bearer_auth(key)
            .json(&payload)
            .send()
            .await
        {
            if let Ok(v) = resp.json::<Value>().await {
                let reply = v.get("choices")
                    .and_then(|c| c.as_array())
                    .and_then(|a| a.first())
                    .and_then(|m| m.get("message"))
                    .and_then(|m| m.get("content"))
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                if !reply.is_empty() {
                    return Json(json!({"reply": reply, "provider": "openrouter"})).into_response();
                }
            }
        }
    }

    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({"error": "no AI provider configured", "reply": ""})),
    )
        .into_response()
}

fn plaid_base() -> String {
    match env("PLAID_ENV").as_deref() {
        Some("production") => "https://production.plaid.com".into(),
        Some("development") => "https://development.plaid.com".into(),
        _ => "https://sandbox.plaid.com".into(),
    }
}

fn user_header(headers: &HeaderMap) -> String {
    headers
        .get("x-pixii-user")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "guest@getpixii.ai".into())
}

// POST /api/plaid/link_token  — create a Plaid Link token for the connect flow
pub async fn plaid_link_token(headers: HeaderMap) -> impl IntoResponse {
    let (Some(cid), Some(secret)) = (env("PLAID_CLIENT_ID"), env("PLAID_SECRET")) else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error": "plaid not configured"}))).into_response();
    };
    let user = user_header(&headers);
    let payload = json!({
        "client_id": cid, "secret": secret,
        "client_name": "Pixii",
        "user": {"client_user_id": user},
        "products": ["transactions"],
        "country_codes": ["US"],
        "language": "en"
    });
    match client().post(format!("{}/link/token/create", plaid_base())).json(&payload).send().await {
        Ok(r) => match r.json::<Value>().await {
            Ok(v) => Json(v).into_response(),
            Err(_) => StatusCode::BAD_GATEWAY.into_response(),
        },
        Err(_) => StatusCode::BAD_GATEWAY.into_response(),
    }
}

// POST /api/plaid/exchange { public_token } — store the per-user access token
pub async fn plaid_exchange(
    State(app): State<App>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    let (Some(cid), Some(secret)) = (env("PLAID_CLIENT_ID"), env("PLAID_SECRET")) else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error": "plaid not configured"}))).into_response();
    };
    let user = user_header(&headers);
    let public_token = body.get("public_token").and_then(|x| x.as_str()).unwrap_or("");
    let payload = json!({"client_id": cid, "secret": secret, "public_token": public_token});
    match client().post(format!("{}/item/public_token/exchange", plaid_base())).json(&payload).send().await {
        Ok(r) => match r.json::<Value>().await {
            Ok(v) => {
                if let Some(tok) = v.get("access_token").and_then(|x| x.as_str()) {
                    let _ = sqlx::query("UPDATE users SET plaid_token=$2 WHERE email=$1")
                        .bind(&user).bind(tok).execute(&app.db).await;
                    return Json(json!({"ok": true})).into_response();
                }
                (StatusCode::BAD_GATEWAY, Json(v)).into_response()
            }
            Err(_) => StatusCode::BAD_GATEWAY.into_response(),
        },
        Err(_) => StatusCode::BAD_GATEWAY.into_response(),
    }
}

async fn plaid_token_for(app: &App, user: &str) -> Option<String> {
    sqlx::query("SELECT plaid_token FROM users WHERE email=$1")
        .bind(user)
        .fetch_optional(&app.db)
        .await
        .ok()
        .flatten()
        .and_then(|r| r.try_get::<Option<String>, _>("plaid_token").ok().flatten())
}

// GET /api/plaid/accounts — live balances for the linked item
pub async fn plaid_accounts(State(app): State<App>, headers: HeaderMap) -> impl IntoResponse {
    let user = user_header(&headers);
    let (Some(cid), Some(secret), Some(tok)) =
        (env("PLAID_CLIENT_ID"), env("PLAID_SECRET"), plaid_token_for(&app, &user).await)
    else {
        return Json(json!({"connected": false, "items": []})).into_response();
    };
    let payload = json!({"client_id": cid, "secret": secret, "access_token": tok});
    match client().post(format!("{}/accounts/balance/get", plaid_base())).json(&payload).send().await {
        Ok(r) => match r.json::<Value>().await {
            Ok(v) => Json(json!({"connected": true, "raw": v})).into_response(),
            Err(_) => StatusCode::BAD_GATEWAY.into_response(),
        },
        Err(_) => StatusCode::BAD_GATEWAY.into_response(),
    }
}
