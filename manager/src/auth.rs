use axum::{
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Admin,
    Reader,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Admin => write!(f, "Admin"),
            Role::Reader => write!(f, "Reader"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub username: String,
    pub role: Role,
    pub auth_enabled: bool,
}

impl AuthUser {
    pub fn is_admin(&self) -> bool {
        self.role == Role::Admin
    }

    pub fn is_reader(&self) -> bool {
        self.role == Role::Reader
    }

    pub fn role_name(&self) -> String {
        self.role.to_string()
    }
}

#[derive(Debug)]
pub struct AuthError(pub String);

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        (
            StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "success": false,
                "message": self.0
            })),
        )
            .into_response()
    }
}

impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);
        let auth_config = &app_state.config.manager.auth;

        if !auth_config.enabled {
            return Ok(AuthUser {
                username: "anonymous".to_string(),
                role: Role::Admin,
                auth_enabled: false,
            });
        }

        // Extract username header
        let username = parts
            .headers
            .get(&auth_config.user_header)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("anonymous")
            .trim()
            .to_string();

        // Extract role header
        let role_str = parts
            .headers
            .get(&auth_config.role_header)
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        // Parse role string (comma separated support)
        let roles: Vec<&str> = role_str.split(',').map(|s| s.trim()).collect();

        let has_admin = roles
            .iter()
            .any(|&r| r.eq_ignore_ascii_case(&auth_config.admin_role));
        let has_reader = roles
            .iter()
            .any(|&r| r.eq_ignore_ascii_case(&auth_config.reader_role));

        let role = if has_admin {
            Role::Admin
        } else if has_reader {
            Role::Reader
        } else {
            // Default to Reader when auth is enabled but no matching role header found
            Role::Reader
        };

        Ok(AuthUser {
            username: if username.is_empty() {
                "anonymous".to_string()
            } else {
                username
            },
            role,
            auth_enabled: true,
        })
    }
}

pub fn inject_auth_context(context: &mut crate::Context, auth_user: &AuthUser) {
    context.insert("auth_enabled", &auth_user.auth_enabled);
    context.insert("current_user", &auth_user.username);
    context.insert("user_role", &auth_user.role_name());
    context.insert("is_admin", &auth_user.is_admin());
    context.insert("is_reader", &auth_user.is_reader());
}
