use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use base64::{engine::general_purpose, Engine as _};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};

use crate::config::AuthConfig;

// HTTP Basic Auth middleware
pub struct BasicAuth {
    config: AuthConfig,
}

impl BasicAuth {
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }

    fn check_auth(&self, auth_header: Option<&str>) -> bool {
        let auth_header = match auth_header {
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
        username == self.config.username
            && bcrypt::verify(password, &self.config.password_hash).unwrap_or(false)
    }
}

impl<S, B> Transform<S, ServiceRequest> for BasicAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = BasicAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(BasicAuthMiddleware {
            service,
            config: self.config.clone(),
        }))
    }
}

pub struct BasicAuthMiddleware<S> {
    service: S,
    config: AuthConfig,
}

impl<S, B> Service<ServiceRequest> for BasicAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Skip auth if disabled in config
        if !self.config.enabled {
            let fut = self.service.call(req);
            return Box::pin(async move {
                let res = fut.await?;
                Ok(res.map_into_left_body())
            });
        }

        let auth_header = req
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok());

        let auth = BasicAuth::new(self.config.clone());
        let is_authenticated = auth.check_auth(auth_header);

        if !is_authenticated {
            let response = HttpResponse::Unauthorized()
                .insert_header(("WWW-Authenticate", "Basic realm=\"Black Box\""))
                .finish()
                .map_into_right_body();

            return Box::pin(async { Ok(ServiceResponse::new(req.into_parts().0, response)) });
        }

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;
            Ok(res.map_into_left_body())
        })
    }
}
