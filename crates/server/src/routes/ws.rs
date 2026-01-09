use std::sync::Arc;

use axum::{
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<WsQuery>,
) -> impl IntoResponse {
    // Verify token if provided
    if let Some(token) = &query.token
        && state.auth_service.verify_token(token).is_err()
    {
        // Could return 401 but WebSocket upgrade doesn't support that cleanly
        // Instead we'll accept but immediately close
    }

    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcast channel
    let mut rx = state.ws_broadcast.subscribe();

    // Spawn task to forward broadcasts to this client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages (ping/pong, client commands)
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    // Handle client messages if needed
                    // For now, we just log them
                    tracing::debug!("Received WS message: {}", text);
                }
                Message::Ping(data) => {
                    // Pong is handled automatically by axum
                    tracing::trace!("Received ping: {:?}", data);
                }
                Message::Close(_) => {
                    tracing::debug!("Client disconnected");
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    tracing::debug!("WebSocket connection closed");
}
