use crate::AppState;
use crate::api::models::ErrorResponse;
use crate::auth::{User, PERM_SERVERS_FILES};
use axum::{
    Router,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::IntoResponse,
    routing::{delete, get},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_util::io::ReaderStream;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:id/files",          get(list_files).delete(delete_file))
        .route("/:id/files/content",  get(get_file_content).put(put_file_content))
        .route("/:id/files/upload",   axum::routing::post(upload_file))
        .route("/:id/files/download", get(download_file))
        .route("/:id/worlds",         get(list_worlds))
        .route("/:id/worlds/:name",   delete(delete_world))
        .route("/:id/worlds/:name/download", get(download_world))
}

#[derive(Deserialize)]
struct FilePath {
    path: Option<String>,
}

#[derive(Serialize)]
struct FileEntry {
    name:     String,
    path:     String,
    is_dir:   bool,
    size:     u64,
    modified: String,
}

fn server_root(server_id: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("/servers/{}", server_id))
}

fn safe_path(server_id: &str, rel: &str) -> Option<std::path::PathBuf> {
    let root = server_root(server_id).canonicalize().ok()
        .unwrap_or_else(|| server_root(server_id));

    let joined = if rel.is_empty() || rel == "/" {
        root.clone()
    } else {
        let rel = rel.trim_start_matches('/');
        root.join(rel)
    };

    let resolved = joined.canonicalize().ok().unwrap_or(joined.clone());

    if !resolved.starts_with(&root) && !joined.starts_with(&root) {
        return None;
    }

    Some(joined)
}

async fn list_files(
    State(_state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Query(q): Query<FilePath>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_FILES) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.files"))).into_response();
    }
    let rel = q.path.unwrap_or_default();
    let dir = match safe_path(&id, &rel) {
        Some(p) => p,
        None => return (StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Invalid path"))).into_response(),
    };

    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(e) => e,
        Err(e) => return (StatusCode::NOT_FOUND, Json(ErrorResponse::new(format!("Cannot read directory: {e}")))).into_response(),
    };

    let root = server_root(&id);
    let mut files: Vec<FileEntry> = Vec::new();

    while let Ok(Some(entry)) = entries.next_entry().await {
        let meta = match entry.metadata().await {
            Ok(m) => m,
            Err(_) => continue,
        };
        let name = entry.file_name().to_string_lossy().to_string();
        let abs  = entry.path();
        let rel_path = abs.strip_prefix(&root)
            .map(|p| format!("/{}", p.to_string_lossy()))
            .unwrap_or_default();

        let modified = meta.modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs().to_string())
            .unwrap_or_default();

        files.push(FileEntry {
            name,
            path: rel_path,
            is_dir: meta.is_dir(),
            size: if meta.is_file() { meta.len() } else { 0 },
            modified,
        });
    }

    files.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Json(files).into_response()
}

async fn get_file_content(
    State(_state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Query(q): Query<FilePath>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_FILES) {
        return (StatusCode::FORBIDDEN, "Missing permission: servers.files".to_string()).into_response();
    }
    let rel = q.path.unwrap_or_default();
    let path = match safe_path(&id, &rel) {
        Some(p) => p,
        None => return (StatusCode::BAD_REQUEST, "Invalid path".to_string()).into_response(),
    };

    match tokio::fs::read_to_string(&path).await {
        Ok(content) => (StatusCode::OK, content).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, format!("Cannot read file: {e}")).into_response(),
    }
}

async fn put_file_content(
    State(_state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Query(q): Query<FilePath>,
    body: String,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_FILES) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.files"))).into_response();
    }
    let rel = q.path.unwrap_or_default();
    let path = match safe_path(&id, &rel) {
        Some(p) => p,
        None => return (StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Invalid path"))).into_response(),
    };

    if let Some(parent) = path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }

    match tokio::fs::write(&path, body).await {
        Ok(_)  => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(format!("Write failed: {e}")))).into_response(),
    }
}

async fn delete_file(
    State(_state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Query(q): Query<FilePath>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_FILES) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.files"))).into_response();
    }
    let rel = q.path.unwrap_or_default();
    let path = match safe_path(&id, &rel) {
        Some(p) => p,
        None => return (StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Invalid path"))).into_response(),
    };

    let meta = match tokio::fs::metadata(&path).await {
        Ok(m)  => m,
        Err(_) => return (StatusCode::NOT_FOUND, Json(ErrorResponse::new("Path not found"))).into_response(),
    };

    let result = if meta.is_dir() {
        tokio::fs::remove_dir_all(&path).await
    } else {
        tokio::fs::remove_file(&path).await
    };

    match result {
        Ok(_)  => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(format!("Delete failed: {e}")))).into_response(),
    }
}

async fn download_file(
    State(_state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Query(q): Query<FilePath>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_FILES) {
        return (StatusCode::FORBIDDEN, "Missing permission: servers.files".to_string()).into_response();
    }
    let rel = q.path.unwrap_or_default();
    let path = match safe_path(&id, &rel) {
        Some(p) => p,
        None => return (StatusCode::BAD_REQUEST, "Invalid path".to_string()).into_response(),
    };

    let file = match tokio::fs::File::open(&path).await {
        Ok(f)  => f,
        Err(e) => return (StatusCode::NOT_FOUND, format!("File not found: {e}")).into_response(),
    };

    let filename = path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "download".to_string());

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename)).unwrap(),
    );
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));

    let stream = ReaderStream::new(file);
    (headers, Body::from_stream(stream)).into_response()
}

async fn upload_file(
    State(_state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
    Query(q): Query<FilePath>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_FILES) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.files"))).into_response();
    }
    let dir_rel = q.path.unwrap_or_default();
    let dir = match safe_path(&id, &dir_rel) {
        Some(p) => p,
        None => return (StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Invalid path"))).into_response(),
    };

    let _ = tokio::fs::create_dir_all(&dir).await;

    while let Ok(Some(field)) = multipart.next_field().await {
        let filename = field.file_name()
            .map(|n| n.to_string())
            .unwrap_or_else(|| "upload".to_string());

        let sanitized: String = filename.chars()
            .filter(|c| c.is_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
            .collect();

        if sanitized.is_empty() { continue; }

        let dest = dir.join(&sanitized);
        if let Ok(bytes) = field.bytes().await {
            let _ = tokio::fs::write(&dest, bytes).await;
        }
    }

    StatusCode::NO_CONTENT.into_response()
}

#[derive(Serialize)]
struct WorldInfo {
    name: String,
    size: u64,
}

const WORLD_DIRS: &[&str] = &["world", "world_nether", "world_the_end", "DIM-1", "DIM1"];

async fn list_worlds(
    State(_state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_FILES) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.files"))).into_response();
    }
    let root = format!("/servers/{}", id);
    let mut worlds: Vec<WorldInfo> = Vec::new();

    for name in WORLD_DIRS {
        let path = std::path::PathBuf::from(format!("{}/{}", root, name));
        if tokio::fs::metadata(&path).await.map(|m| m.is_dir()).unwrap_or(false) {
            let size = tokio::task::spawn_blocking({
                let p = path.clone();
                move || dir_size(&p)
            }).await.unwrap_or(0);
            worlds.push(WorldInfo { name: name.to_string(), size });
        }
    }

    Json(worlds).into_response()
}

async fn delete_world(
    State(_state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path((id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_FILES) {
        return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("Missing permission: servers.files"))).into_response();
    }
    if !WORLD_DIRS.contains(&name.as_str()) {
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Not a recognised world directory"))).into_response();
    }

    let path = std::path::PathBuf::from(format!("/servers/{}/{}", id, name));
    if !tokio::fs::metadata(&path).await.map(|m| m.is_dir()).unwrap_or(false) {
        return (StatusCode::NOT_FOUND, Json(ErrorResponse::new("World directory not found"))).into_response();
    }

    match tokio::fs::remove_dir_all(&path).await {
        Ok(_)  => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new(format!("Delete failed: {e}")))).into_response(),
    }
}

async fn download_world(
    State(_state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    Path((id, name)): Path<(String, String)>,
) -> impl IntoResponse {
    if !user.has_permission(PERM_SERVERS_FILES) {
        return (StatusCode::FORBIDDEN, "Missing permission: servers.files".to_string()).into_response();
    }
    if !WORLD_DIRS.contains(&name.as_str()) {
        return (StatusCode::BAD_REQUEST, "Not a recognised world directory".to_string()).into_response();
    }

    let world_path = format!("/servers/{}/{}", id, name);
    let zip_path   = format!("/tmp/novabox-{}-{}.zip", id, name);

    let result = tokio::task::spawn_blocking({
        let wp = world_path.clone();
        let zp = zip_path.clone();
        move || zip_dir(&wp, &zp)
    }).await;

    match result {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
        Err(e)     => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }

    let file = match tokio::fs::File::open(&zip_path).await {
        Ok(f)  => f,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Could not open zip: {e}")).into_response(),
    };

    let filename = format!("{}-{}.zip", id, name);
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename)).unwrap(),
    );
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/zip"));

    let zip_path_clone = zip_path.clone();
    let stream = ReaderStream::new(file);
    let body   = Body::from_stream(stream);

    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        let _ = tokio::fs::remove_file(&zip_path_clone).await;
    });

    (headers, body).into_response()
}

fn dir_size(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let p = entry.path();
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_file() {
                total = total.saturating_add(meta.len());
            } else if meta.is_dir() {
                stack.push(p);
            }
        }
    }
    total
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
