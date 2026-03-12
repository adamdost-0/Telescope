//! WebSocket handler for streaming (log streams, connection state updates).
//!
//! Placeholder — full streaming implementation planned for a future milestone.

use axum::extract::ws::{Message, WebSocket};
use tracing::info;

/// Handle an inbound WebSocket connection.
///
/// Currently sends a welcome message and closes. Real streaming
/// (log follow, connection-state push) will be added in a follow-up.
pub async fn handle_ws(mut socket: WebSocket) {
    if socket
        .send(Message::Text(
            r#"{"type":"welcome","message":"Telescope Hub WebSocket"}"#.into(),
        ))
        .await
        .is_err()
    {
        return;
    }

    // Echo loop — keeps the socket alive for health-check tooling.
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(t) => {
                info!("ws recv: {}", t);
                if socket
                    .send(Message::Text(format!(r#"{{"echo":{}}}"#, t).into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
