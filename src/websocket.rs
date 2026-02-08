use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;
use crate::env::EnvironmentManager;
use crate::parser::WebSocketRequest;

pub struct WebSocketClient {
    env_manager: EnvironmentManager,
}

impl WebSocketClient {
    pub fn new(env_manager: EnvironmentManager) -> Self {
        Self { env_manager }
    }

    pub async fn execute_request(
        &self,
        request: &WebSocketRequest,
        env_name: Option<&str>,
    ) -> Result<()> {
        let env_name = env_name.unwrap_or("default");

        // Resolve URI with environment variables
        let uri = self.env_manager.resolve_string(env_name, &request.uri);

        // Parse URL
        let url = Url::parse(&uri)
            .with_context(|| format!("Invalid WebSocket URL: {}", uri))?;

        println!("Connecting to WebSocket: {}", url);

        // Connect to WebSocket
        let (ws_stream, _) = connect_async(url)
            .await
            .context("Failed to connect to WebSocket")?;

        let (mut write, mut read) = ws_stream.split();

        // Send messages
        for message in &request.messages {
            // Wait for server responses if needed
            for _ in 0..message.wait_for_server {
                if let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            println!("Received: {}", text);
                        }
                        Ok(Message::Binary(data)) => {
                            println!("Received binary: {} bytes", data.len());
                        }
                        Ok(Message::Close(_)) => {
                            println!("Connection closed by server");
                            return Ok(());
                        }
                        Err(e) => {
                            eprintln!("Error receiving message: {}", e);
                            return Err(e.into());
                        }
                        _ => {}
                    }
                }
            }

            // Resolve message content with environment variables
            let content = self.env_manager.resolve_string(env_name, &message.content);

            // Send message
            println!("Sending: {}", content);
            write.send(Message::Text(content))
                .await
                .context("Failed to send WebSocket message")?;

            // Wait for response (if not waiting for multiple)
            if message.wait_for_server == 0 {
                if let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            println!("Received: {}", text);
                        }
                        Ok(Message::Binary(data)) => {
                            println!("Received binary: {} bytes", data.len());
                        }
                        Ok(Message::Close(_)) => {
                            println!("Connection closed by server");
                            return Ok(());
                        }
                        Err(e) => {
                            eprintln!("Error receiving message: {}", e);
                            return Err(e.into());
                        }
                        _ => {}
                    }
                }
            }
        }

        // Keep connection alive and listen for more messages
        println!("Listening for messages (press Ctrl+C to exit)...");
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    println!("Received: {}", text);
                }
                Ok(Message::Binary(data)) => {
                    println!("Received binary: {} bytes", data.len());
                }
                Ok(Message::Close(_)) => {
                    println!("Connection closed by server");
                    break;
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
