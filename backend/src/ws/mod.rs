use crate::AppState;
use crate::auth::PERM_SERVERS_CONSOLE;
use axum::{
    Router,
    extract::{Path, Query, State, WebSocketUpgrade, ws::{Message, WebSocket}},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use futures_util::StreamExt;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
struct WsQuery {
    token: Option<String>,
}

pub fn router<S>(state: Arc<AppState>) -> Router<S> {
    Router::new()
        .route("/console/:server_id", get(console_ws))
        .with_state(state)
}

async fn console_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
    Query(q): Query<WsQuery>,
) -> impl IntoResponse {
    let token = match q.token {
        Some(t) if !t.is_empty() => t,
        _ => return StatusCode::UNAUTHORIZED.into_response(),
    };

    let user_id = match state.auth.resolve_token(&token).await {
        Some(uid) => uid,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };

    let user = match state.auth.find_by_id(&user_id).await {
        Some(u) => u,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };

    if !user.has_permission(PERM_SERVERS_CONSOLE) {
        return StatusCode::FORBIDDEN.into_response();
    }

    ws.on_upgrade(move |socket| handle_console(socket, state, server_id))
}

async fn handle_console(mut socket: WebSocket, state: Arc<AppState>, server_id: String) {
    let tx = state.get_or_create_log_channel(&server_id).await;
    let mut rx = tx.subscribe();

    let history = state.recent_log_lines(&server_id, 1000).await;
    for line in history {
        if socket.send(Message::Text(line.into())).await.is_err() {
            return;
        }
    }

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(line) => {
                        if socket.send(Message::Text(line.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
            msg = socket.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}
