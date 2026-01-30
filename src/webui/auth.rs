use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{engine::general_purpose, Engine as _};

fn check_auth(headers: &HeaderMap, config: &crate::config::AuthConfig) -> bool {
    let auth_header = match headers.get("Authorization").and_then(|h| h.to_str().ok()) {
        Some(h) => h,
        None => return false,
    };

    // Check if it starts with "Basic "
    if !auth_header.starts_with("Basic ") {
        return false;
    }

    // Decode base64 credentials
    let credentials = match general_purpose::STANDARD.decode(&auth_header[6..]) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let credentials_str = match String::from_utf8(credentials) {
        Ok(s) => s,
        Err(_) => return false,
    };

    // Split username:password
    let parts: Vec<&str> = credentials_str.splitn(2, ':').collect();
    if parts.len() != 2 {
        return false;
    }

    let (username, password) = (parts[0], parts[1]);

    // Verify username and password hash
    username == config.username
        && bcrypt::verify(password, &config.password_hash).unwrap_or(false)
}

pub async fn basic_auth_middleware(
    State(state): State<super::routes::AppState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    // Skip auth if disabled in config
    if !state.config.auth.enabled {
        return next.run(req).await;
    }

    if !check_auth(req.headers(), &state.config.auth) {
        return (
            StatusCode::UNAUTHORIZED,
            [("WWW-Authenticate", "Basic realm=\"Black Box\"")],
        )
            .into_response();
    }

    next.run(req).await
}
