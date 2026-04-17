use crate::auth::{hash_password, verify_password, Role, User, DEFAULT_USER_PERMISSIONS};
use crate::AppState;
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/setup",  get(setup_status).post(do_setup))
        .route("/login",  post(login))
        .route("/logout", post(logout))
        .route("/me",     get(me))
}

#[derive(Serialize)]
struct SetupStatus {
    needs_setup: bool,
}

#[derive(Serialize)]
struct AuthResponse {
    token:    String,
    user:     UserDto,
}

#[derive(Serialize, Clone)]
pub struct UserDto {
    pub id:          String,
    pub username:    String,
    pub role:        String,
    pub permissions: Vec<String>,
    pub settings:    serde_json::Value,
    pub created_at:  String,
}

impl From<User> for UserDto {
    fn from(u: User) -> Self {
        Self {
            id:          u.id,
            username:    u.username,
            role:        format!("{:?}", u.role).to_lowercase(),
            permissions: u.permissions,
            settings:    u.settings,
            created_at:  u.created_at,
        }
    }
}

async fn setup_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(SetupStatus { needs_setup: state.auth.needs_setup().await })
}

#[derive(Deserialize)]
struct SetupRequest {
    username: String,
    password: String,
}

async fn do_setup(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetupRequest>,
) -> impl IntoResponse {
    if !state.auth.needs_setup().await {
        return (StatusCode::CONFLICT, "Setup already complete").into_response();
    }
    let username = body.username.trim().to_string();
    if username.is_empty() || body.password.len() < 4 {
        return (StatusCode::BAD_REQUEST, "Username and password (min 4 chars) required").into_response();
    }
    let user = User {
        id:            Uuid::new_v4().to_string(),
        username,
        password_hash: hash_password(&body.password),
        role:          Role::Admin,
        permissions:   vec![],
        settings:      serde_json::Value::Object(Default::default()),
        created_at:    chrono::Utc::now().to_rfc3339(),
    };
    if let Err(e) = state.auth.save_users(&[user.clone()]).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Save failed: {e}")).into_response();
    }
    let token = state.auth.create_session(user.id.clone()).await;
    Json(AuthResponse { token, user: user.into() }).into_response()
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> impl IntoResponse {
    let Some(user) = state.auth.find_by_username(&body.username).await else {
        return (StatusCode::UNAUTHORIZED, "Invalid credentials").into_response();
    };
    if !verify_password(&body.password, &user.password_hash) {
        return (StatusCode::UNAUTHORIZED, "Invalid credentials").into_response();
    }
    let token = state.auth.create_session(user.id.clone()).await;
    Json(AuthResponse { token, user: user.into() }).into_response()
}

async fn logout(
    State(state): State<Arc<AppState>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> impl IntoResponse {
    state.auth.revoke_token(auth.token()).await;
    StatusCode::NO_CONTENT
}

async fn me(
    State(state): State<Arc<AppState>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> impl IntoResponse {
    let Some(user_id) = state.auth.resolve_token(auth.token()).await else {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    };

    let Some(user) = state.auth.find_by_id(&user_id).await else {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    };

    Json(UserDto::from(user)).into_response()
}

pub fn default_permissions() -> Vec<String> {
    DEFAULT_USER_PERMISSIONS.iter().map(|s| s.to_string()).collect()
}
