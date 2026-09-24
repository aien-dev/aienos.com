use std::{
    collections::{HashMap, VecDeque},
    env,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use email_address::EmailAddress;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

const CONSENT_VERSION: &str = "2026-09-23-v1";
const MAX_REQUESTS_PER_IP: usize = 24;
const RATE_WINDOW: Duration = Duration::from_secs(3600);

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Connection>>,
    attempts: Arc<Mutex<HashMap<std::net::IpAddr, VecDeque<Instant>>>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Signup {
    email: String,
    consent: bool,
    consent_version: String,
    #[serde(default)]
    website: String,
}

#[derive(Serialize)]
struct Reply {
    ok: bool,
    message: &'static str,
}

fn allowed_origin(headers: &HeaderMap) -> Option<HeaderValue> {
    let origin = headers.get(header::ORIGIN)?;
    let text = origin.to_str().ok()?;
    match text {
        "https://aienos.com" | "https://www.aienos.com" => Some(origin.clone()),
        _ => None,
    }
}

fn response(
    status: StatusCode,
    ok: bool,
    message: &'static str,
    origin: Option<HeaderValue>,
) -> Response {
    let mut result = (status, Json(Reply { ok, message })).into_response();
    let headers = result.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    if let Some(origin) = origin {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
        headers.insert(header::VARY, HeaderValue::from_static("Origin"));
    }
    result
}

async fn preflight(headers: HeaderMap) -> Response {
    let Some(origin) = allowed_origin(&headers) else {
        return response(StatusCode::FORBIDDEN, false, "origin not allowed", None);
    };
    let mut result = StatusCode::NO_CONTENT.into_response();
    result
        .headers_mut()
        .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
    result.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("POST, OPTIONS"),
    );
    result.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("Content-Type"),
    );
    result.headers_mut().insert(
        header::ACCESS_CONTROL_MAX_AGE,
        HeaderValue::from_static("600"),
    );
    result
        .headers_mut()
        .insert(header::VARY, HeaderValue::from_static("Origin"));
    result
}

async fn signup(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Signup>,
) -> Response {
    let Some(origin) = allowed_origin(&headers) else {
        return response(StatusCode::FORBIDDEN, false, "origin not allowed", None);
    };
    if headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_none_or(|v| !v.starts_with("application/json"))
    {
        return response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            false,
            "JSON required",
            Some(origin),
        );
    }
    if !body.consent || body.consent_version != CONSENT_VERSION {
        return response(
            StatusCode::BAD_REQUEST,
            false,
            "consent required",
            Some(origin),
        );
    }
    let normalized = body.email.trim().to_ascii_lowercase();
    if normalized.len() > 254 || normalized.parse::<EmailAddress>().is_err() {
        return response(
            StatusCode::BAD_REQUEST,
            false,
            "invalid email",
            Some(origin),
        );
    }
    if !body.website.is_empty() {
        return response(StatusCode::ACCEPTED, true, "received", Some(origin));
    }
    {
        let mut attempts = state.attempts.lock().expect("rate limiter lock");
        let now = Instant::now();
        // The public listener is reached only through the local Tailscale proxy.
        // Prefer its forwarded client address for fair per-client limits.
        let client_ip = if peer.ip().is_loopback() {
            headers
                .get("x-forwarded-for")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.split(',').next())
                .and_then(|value| value.trim().parse().ok())
                .unwrap_or(peer.ip())
        } else {
            peer.ip()
        };
        // If the proxy omits a forwarded address, all visitors share its
        // loopback address. Preserve signup capacity in that case.
        let limit = if client_ip == peer.ip() && peer.ip().is_loopback() {
            2400
        } else {
            MAX_REQUESTS_PER_IP
        };
        let recent = attempts.entry(client_ip).or_default();
        while recent
            .front()
            .is_some_and(|at| now.duration_since(*at) >= RATE_WINDOW)
        {
            recent.pop_front();
        }
        if recent.len() >= limit {
            return response(
                StatusCode::TOO_MANY_REQUESTS,
                false,
                "try again later",
                Some(origin),
            );
        }
        recent.push_back(now);
        // Keep stale IP buckets from growing without bound.
        if attempts.len() > 4096 {
            attempts.retain(|_, events| {
                events
                    .back()
                    .is_some_and(|at| now.duration_since(*at) < RATE_WINDOW)
            });
        }
    }
    let db = state.db.lock().expect("database lock");
    match db.execute(
        "INSERT OR IGNORE INTO signups(email, consent_version) VALUES (?1, ?2)",
        params![normalized, CONSENT_VERSION],
    ) {
        Ok(_) => response(StatusCode::CREATED, true, "received", Some(origin)),
        Err(error) => {
            eprintln!("waitlist database error: {error}");
            response(
                StatusCode::SERVICE_UNAVAILABLE,
                false,
                "temporarily unavailable",
                Some(origin),
            )
        }
    }
}

async fn health() -> Response {
    response(StatusCode::OK, true, "healthy", None)
}

fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/api/signup", post(signup).options(preflight))
        .layer(DefaultBodyLimit::max(4096))
        .with_state(state)
}

fn open_database(path: &str) -> rusqlite::Result<Connection> {
    let db = Connection::open(path)?;
    db.execute_batch(
        "PRAGMA journal_mode=WAL;
         CREATE TABLE IF NOT EXISTS signups (
           email TEXT PRIMARY KEY NOT NULL,
           consent_version TEXT NOT NULL,
           created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
         );",
    )?;
    Ok(db)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = env::var("WAITLIST_DB_PATH").unwrap_or_else(|_| "waitlist.sqlite3".into());
    let bind = env::var("WAITLIST_BIND").unwrap_or_else(|_| "127.0.0.1:18171".into());
    let state = AppState {
        db: Arc::new(Mutex::new(open_database(&db_path)?)),
        attempts: Arc::new(Mutex::new(HashMap::new())),
    };
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    println!("AIENOS waitlist listening on {bind}");
    axum::serve(
        listener,
        router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use tower::ServiceExt;

    async fn request(app: Router, email: &str, origin: &str) -> (StatusCode, HeaderMap) {
        let payload = serde_json::json!({
            "email": email, "consent": true, "consentVersion": CONSENT_VERSION, "website": ""
        });
        let request = Request::post("/api/signup")
            .header(header::ORIGIN, origin)
            .header(header::CONTENT_TYPE, "application/json")
            .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1234))))
            .body(Body::from(payload.to_string()))
            .unwrap();
        let result = app.oneshot(request).await.unwrap();
        let status = result.status();
        let headers = result.headers().clone();
        let _ = to_bytes(result.into_body(), 4096).await.unwrap();
        (status, headers)
    }

    #[tokio::test]
    async fn stores_once_and_rejects_other_origins() {
        let db = Arc::new(Mutex::new(open_database(":memory:").unwrap()));
        let app = router(AppState {
            db: db.clone(),
            attempts: Arc::new(Mutex::new(HashMap::new())),
        });
        assert_eq!(
            request(app.clone(), " TEST@Example.com ", "https://www.aienos.com")
                .await
                .0,
            StatusCode::CREATED
        );
        assert_eq!(
            request(app.clone(), "test@example.com", "https://www.aienos.com")
                .await
                .0,
            StatusCode::CREATED
        );
        assert_eq!(
            request(app, "bad@example.com", "https://evil.example")
                .await
                .0,
            StatusCode::FORBIDDEN
        );
        let db = db.lock().unwrap();
        let count: i64 = db
            .query_row("SELECT count(*) FROM signups", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
