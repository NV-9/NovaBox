use crate::AppState;
use crate::api::models::ErrorResponse;
use crate::auth::{User, PERM_SERVERS_FILES};
use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::IntoResponse,
    routing::{delete, get},
    Extension, Json,
};
use serde::Serialize;
use std::sync::Arc;
use tokio_util::io::ReaderStream;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:id/backups",                get(list_backups).post(create_backup))
        .route("/:id/backups/:name",          delete(delete_backup))
        .route("/:id/backups/:name/download", get(download_backup))
}

fn backup_dir(server_id: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("/app/data/backups/{}", server_id))
}

#[derive(Serialize)]
struct BackupEntry {
    name:       String,
    size:       u64,
    created_at: u64,
}

async fn list_backups(
    State(_state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_FILES) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.files"))).into_response();
    }
    let dir = backup_dir(&id);
    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(e)  => e,
        Err(_) => return Json(Vec::<BackupEntry>::new()).into_response(),
    };

    let mut backups: Vec<BackupEntry> = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let meta = match entry.metadata().await {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() { continue; }
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".zip") { continue; }

        let created_at = meta.created()
            .or_else(|_| meta.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        backups.push(BackupEntry { name, size: meta.len(), created_at });
    }

    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Json(backups).into_response()
}

async fn create_backup(
    State(_state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_FILES) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.files"))).into_response();
    }
    let dir = backup_dir(&id);
    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new(format!("Cannot create backup dir: {e}"))),
        )
            .into_response();
    }

    let ts   = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
    let name = format!("backup-{}.zip", ts);
    let zip_path = dir.join(&name).to_string_lossy().to_string();
    let src  = format!("/servers/{}", id);

    let result = tokio::task::spawn_blocking(move || zip_dir(&src, &zip_path)).await;

    match result {
        Ok(Ok(_)) => {
            let created_at = chrono::Utc::now().timestamp() as u64;
            let size = tokio::fs::metadata(dir.join(&name))
                .await
                .map(|m| m.len())
                .unwrap_or(0);
            Json(BackupEntry { name, size, created_at }).into_response()
        }
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e))).into_response(),
        Err(e)     => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(e.to_string()))).into_response(),
    }
}

async fn delete_backup(
    State(_state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path((id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_FILES) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.files"))).into_response();
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") || !name.ends_with(".zip") {
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Invalid backup name"))).into_response();
    }
    let path = backup_dir(&id).join(&name);
    match tokio::fs::remove_file(&path).await {
        Ok(_)  => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(format!("Delete failed: {e}")))).into_response(),
    }
}

async fn download_backup(
    State(_state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path((id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_FILES) {
        return (StatusCode::FORBIDDEN, "Missing permission: servers.files".to_string()).into_response();
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") || !name.ends_with(".zip") {
        return (StatusCode::BAD_REQUEST, "Invalid backup name".to_string()).into_response();
    }
    let path = backup_dir(&id).join(&name);
    let file = match tokio::fs::File::open(&path).await {
        Ok(f)  => f,
        Err(e) => return (StatusCode::NOT_FOUND, format!("Backup not found: {e}")).into_response(),
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", name)).unwrap(),
    );
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/zip"));
    (headers, Body::from_stream(ReaderStream::new(file))).into_response()
}

fn zip_dir(src: &str, dest: &str) -> Result<(), String> {
    use std::io::Write;

    let file = std::fs::File::create(dest).map_err(|e| format!("Create zip failed: {e}"))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let src_path = std::path::Path::new(src);
    let mut stack = vec![src_path.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let path = entry.path();
            let rel  = path.strip_prefix(src_path)
                .map_err(|e| e.to_string())?
                .to_string_lossy()
                .replace('\\', "/");

            if path.is_dir() {
                stack.push(path);
            } else {
                zip.start_file(&rel, options).map_err(|e| e.to_string())?;
                let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
                zip.write_all(&bytes).map_err(|e| e.to_string())?;
            }
        }
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}
