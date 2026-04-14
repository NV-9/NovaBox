use crate::AppState;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

const MODRINTH_API: &str = "https://api.modrinth.com/v2";

pub fn router<S>(state: Arc<AppState>) -> Router<S> {
    Router::new()
        .route("/search", get(search_mods))
        .route("/project/:id", get(get_project))
        .route("/project/:id/versions", get(get_versions))
        .with_state(state)
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    #[serde(default)]
    loader: String,
    #[serde(default)]
    game_version: String,
    #[serde(default = "default_limit")]
    limit: u32,
    #[serde(default)]
    offset: u32,
}
fn default_limit() -> u32 { 20 }

async fn search_mods(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let client = reqwest::Client::new();

    let mut facets = vec![];
    if !params.loader.is_empty() {
        facets.push(format!(r#"["categories:{}"]"#, params.loader.to_lowercase()));
    }
    if !params.game_version.is_empty() {
        facets.push(format!(r#"["versions:{}"]"#, params.game_version));
    }

    let facets_str = if facets.is_empty() {
        String::new()
    } else {
        format!("[{}]", facets.join(","))
    };

    let mut url = format!(
        "{}/search?query={}&limit={}&offset={}",
        MODRINTH_API, params.q, params.limit, params.offset
    );
    if !facets_str.is_empty() {
        url.push_str(&format!("&facets={}", urlencoding::encode(&facets_str)));
    }

    match client
        .get(&url)
        .header("User-Agent", "NovaBox/0.1")
        .send()
        .await
    {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(json) => Json(json).into_response(),
            Err(e) => (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    }
}

async fn get_project(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let client = reqwest::Client::new();
    let url = format!("{}/project/{}", MODRINTH_API, id);

    match client.get(&url).header("User-Agent", "NovaBox/0.1").send().await {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(json) => Json(json).into_response(),
            Err(e) => (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    }
}

async fn get_versions(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let client = reqwest::Client::new();
    let url = format!("{}/project/{}/version", MODRINTH_API, id);

    match client.get(&url).header("User-Agent", "NovaBox/0.1").send().await {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(json) => Json(json).into_response(),
            Err(e) => (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
        },
        Err(e) => (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    }
}
