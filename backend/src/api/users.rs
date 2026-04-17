use crate::api::auth::{default_permissions, UserDto};
use crate::api::models::ErrorResponse;
use crate::auth::{hash_password, Role, User, ALL_PERMISSIONS};
use crate::AppState;
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/",    get(list_users).post(create_user))
        .route("/:id", get(get_user).put(update_user).delete(delete_user))
        .route("/:id/settings", get(get_settings).put(put_settings))
        .route("/permissions",  get(list_permissions))
}

#[derive(Serialize)]
struct PermissionInfo {
    key:         &'static str,
    label:       &'static str,
    description: &'static str,
}

async fn list_permissions() -> impl IntoResponse {
    let perms: Vec<PermissionInfo> = vec![
        PermissionInfo { key: "servers.view",       label: "View Servers",       description: "See the server list and details" },
        PermissionInfo { key: "servers.create",     label: "Create Servers",     description: "Spin up new Minecraft servers" },
        PermissionInfo { key: "servers.delete",     label: "Delete Servers",     description: "Permanently delete servers" },
        PermissionInfo { key: "servers.power",      label: "Power Control",      description: "Start, stop, and restart servers" },
        PermissionInfo { key: "servers.console",    label: "Console",            description: "View and send console commands" },
        PermissionInfo { key: "servers.files",      label: "File Browser",       description: "Browse and edit server files" },
        PermissionInfo { key: "servers.settings",   label: "Server Settings",    description: "Change server configuration" },
        PermissionInfo { key: "servers.players",    label: "Player Monitoring",  description: "View connected players" },
        PermissionInfo { key: "servers.moderation", label: "Moderation",         description: "Whitelist, ban, and op management" },
        PermissionInfo { key: "servers.modrinth",   label: "Modrinth",           description: "Browse and install mods" },
        PermissionInfo { key: "analytics.view",     label: "Analytics",          description: "View analytics and metrics" },
        PermissionInfo { key: "mods.browse",        label: "Mod Browser",        description: "Global mod browser access" },
    ];
    Json(perms)
}

async fn list_users(
    State(state): State<Arc<AppState>>,
    Extension(caller): Extension<User>,
) -> impl IntoResponse {
    if caller.role != Role::Admin {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Admin only"))).into_response();
    }
    let users: Vec<UserDto> = state.auth.load_users().await.into_iter().map(UserDto::from).collect();
    Json(users).into_response()
}

async fn get_user(
    State(state): State<Arc<AppState>>,
    Extension(caller): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if caller.role != Role::Admin && caller.id != id {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Admin only"))).into_response();
    }
    match state.auth.find_by_id(&id).await {
        Some(u) => Json(UserDto::from(u)).into_response(),
        None    => (StatusCode::NOT_FOUND, Json(ErrorResponse::new("User not found"))).into_response(),
    }
}

#[derive(Deserialize)]
struct CreateUserRequest {
    username:    String,
    password:    String,
    role:        Option<String>,
    permissions: Option<Vec<String>>,
}

async fn create_user(
    State(state): State<Arc<AppState>>,
    Extension(caller): Extension<User>,
    Json(body): Json<CreateUserRequest>,
) -> impl IntoResponse {
    if caller.role != Role::Admin {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Admin only"))).into_response();
    }
    let username = body.username.trim().to_string();
    if username.is_empty() || body.password.len() < 4 {
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Username and password (min 4 chars) required"))).into_response();
    }
    let mut users = state.auth.load_users().await;
    if users.iter().any(|u| u.username.to_lowercase() == username.to_lowercase()) {
        return (StatusCode::CONFLICT, Json(ErrorResponse::new("Username already taken"))).into_response();
    }
    let role = match body.role.as_deref() {
        Some("admin") => Role::Admin,
        _             => Role::User,
    };
    let permissions = body.permissions.unwrap_or_else(default_permissions);
    let permissions: Vec<String> = permissions.into_iter()
        .filter(|p| ALL_PERMISSIONS.contains(&p.as_str()))
        .collect();

    let user = User {
        id:            Uuid::new_v4().to_string(),
        username,
        password_hash: hash_password(&body.password),
        role,
        permissions,
        settings:      serde_json::Value::Object(Default::default()),
        created_at:    chrono::Utc::now().to_rfc3339(),
    };
    users.push(user.clone());
    if let Err(e) = state.auth.save_users(&users).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(format!("Save failed: {e}")))).into_response();
    }
    (StatusCode::CREATED, Json(UserDto::from(user))).into_response()
}

#[derive(Deserialize)]
struct UpdateUserRequest {
    username:    Option<String>,
    password:    Option<String>,
    role:        Option<String>,
    permissions: Option<Vec<String>>,
}

async fn update_user(
    State(state): State<Arc<AppState>>,
    Extension(caller): Extension<User>,
    Path(id): Path<String>,
    Json(body): Json<UpdateUserRequest>,
) -> impl IntoResponse {
    if caller.role != Role::Admin && caller.id != id {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Admin only"))).into_response();
    }
    let mut users = state.auth.load_users().await;
    let Some(user) = users.iter_mut().find(|u| u.id == id) else {
        return (StatusCode::NOT_FOUND, Json(ErrorResponse::new("User not found"))).into_response();
    };
    if let Some(name) = body.username {
        let name = name.trim().to_string();
        if !name.is_empty() {
            user.username = name;
        }
    }
    if let Some(pw) = body.password {
        if pw.len() >= 4 {
            user.password_hash = hash_password(&pw);
        }
    }
    if caller.role == Role::Admin {
        if let Some(role) = body.role {
            user.role = match role.as_str() {
                "admin" => Role::Admin,
                _       => Role::User,
            };
        }
        if let Some(perms) = body.permissions {
            user.permissions = perms.into_iter()
                .filter(|p| ALL_PERMISSIONS.contains(&p.as_str()))
                .collect();
        }
    }
    let dto = UserDto::from(user.clone());
    if let Err(e) = state.auth.save_users(&users).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(format!("Save failed: {e}")))).into_response();
    }
    Json(dto).into_response()
}

async fn delete_user(
    State(state): State<Arc<AppState>>,
    Extension(caller): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if caller.role != Role::Admin {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Admin only"))).into_response();
    }
    if caller.id == id {
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Cannot delete your own account"))).into_response();
    }
    let mut users = state.auth.load_users().await;
    let before = users.len();
    users.retain(|u| u.id != id);
    if users.len() == before {
        return (StatusCode::NOT_FOUND, Json(ErrorResponse::new("User not found"))).into_response();
    }
    if let Err(e) = state.auth.save_users(&users).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(format!("Save failed: {e}")))).into_response();
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn get_settings(
    State(state): State<Arc<AppState>>,
    Extension(caller): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if caller.role != Role::Admin && caller.id != id {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Forbidden"))).into_response();
    }
    match state.auth.find_by_id(&id).await {
        Some(u) => Json(u.settings).into_response(),
        None    => (StatusCode::NOT_FOUND, Json(ErrorResponse::new("User not found"))).into_response(),
    }
}

async fn put_settings(
    State(state): State<Arc<AppState>>,
    Extension(caller): Extension<User>,
    Path(id): Path<String>,
    Json(new_settings): Json<serde_json::Value>,
) -> impl IntoResponse {
    if caller.role != Role::Admin && caller.id != id {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Forbidden"))).into_response();
    }
    let mut users = state.auth.load_users().await;
    let Some(user) = users.iter_mut().find(|u| u.id == id) else {
        return (StatusCode::NOT_FOUND, Json(ErrorResponse::new("User not found"))).into_response();
    };
    user.settings = new_settings;
    if let Err(e) = state.auth.save_users(&users).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(format!("Save failed: {e}")))).into_response();
    }
    StatusCode::NO_CONTENT.into_response()
}
