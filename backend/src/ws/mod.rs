use crate::AppState;
use axum::{
    Router,
    extract::{Path, State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::IntoResponse,
    routing::get,
};
use futures_util::StreamExt;
use std::sync::Arc;

pub fn router<S>(state: Arc<AppState>) -> Router<S> {
    Router::new()
        .route("/console/:server_id", get(console_ws))
        .with_state(state)
}

async fn console_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Path(server_id): Path<String>,
) -> impl IntoResponse {
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
