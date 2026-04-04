pub mod config;
pub mod export;
pub mod monitor;
pub mod status;
pub mod systemd;

/// Apply optional HTTP basic auth to a request builder.
pub fn with_auth(
    req: reqwest::blocking::RequestBuilder,
    username: &Option<String>,
    password: &Option<String>,
) -> reqwest::blocking::RequestBuilder {
    if let (Some(u), Some(p)) = (username, password) {
        req.basic_auth(u, Some(p))
    } else {
        req
    }
}
