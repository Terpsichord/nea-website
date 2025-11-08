use std::path::Path;

use axum::extract::ws::{Message, WebSocket};
use bollard::Docker;
use ws_messages::ClientMessage;

pub struct WebSocketHandler {
    ws: WebSocket,
    docker: Docker,  
}

impl WebSocketHandler {
    pub async fn handle(ws: WebSocket) {
        let handler = Self::new(ws);

        while let Some(recv) = ws.recv().await {
            match recv {
                Ok(Message::Text(msg)) => handler.process_message(&msg),
                Ok(Message::Binary(_)) => return Err(""),
                Err(err) => return err,
                _ => {}
            }
        }
    }

    fn new(ws: WebSocket) -> Self {
        Self {
            ws,
            docker: Docker::connect_with_local_defaults(),
        }
    }

    fn process_message(msg: &str) -> Result<(), serde_json::Error> {
        let client_msg: ClientMessage = serde_json::from_str(msg)?;

        match client_msg {
            ClientMessage::ReadFile { path } => read_file(&path),
        }
    }

    fn read_file(&self, path: &Path) {}
}
