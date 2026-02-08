//! RSocket client for executing RSocket requests from .http files.
//! Uses WebSocket transport (ws://, wss://) for cross-platform support.

use anyhow::{Context, Result};
use rsocket_rust::prelude::*;
use rsocket_rust::utils::EchoRSocket;
use rsocket_rust_transport_websocket::WebsocketClientTransport;

use crate::env::EnvironmentManager;
use crate::parser::RSocketRequest;

/// Normalizes RSocket URI for WebSocket transport.
/// Supports: ws://, wss://, or rs://host:port (converted to ws://host:port)
pub(crate) fn uri_to_transport_addr(uri: &str) -> Result<String> {
    let s = uri.trim();
    if s.starts_with("ws://") || s.starts_with("wss://") {
        Ok(s.to_string())
    } else if s.starts_with("rs://") {
        Ok(format!("ws://{}", &s["rs://".len()..]))
    } else if s.starts_with("tcp://") {
        Ok(format!("ws://{}", &s["tcp://".len()..]))
    } else if s.contains("://") {
        anyhow::bail!("RSocket expects ws://, wss://, rs://, or tcp:// scheme");
    } else {
        Ok(format!("ws://{}", s))
    }
}

pub struct RSocketClient {
    env_manager: EnvironmentManager,
}

impl RSocketClient {
    pub fn new(env_manager: EnvironmentManager) -> Self {
        Self { env_manager }
    }

    pub async fn execute_request(
        &self,
        request: &RSocketRequest,
        env_name: Option<&str>,
    ) -> Result<()> {
        let env_name = env_name.unwrap_or("default");

        let uri = self.env_manager.resolve_string(env_name, &request.uri);
        let addr = uri_to_transport_addr(&uri).with_context(|| format!("Invalid RSocket URI: {}", uri))?;

        println!("Connecting to RSocket: {} ({})", uri, addr);

        let client = RSocketFactory::connect()
            .transport(WebsocketClientTransport::from(addr.as_str()))
            .acceptor(Box::new(|| Box::new(EchoRSocket)))
            .start()
            .await
            .context("Failed to connect to RSocket")?;

        for message in &request.messages {
            for _ in 0..message.wait_for_server {
                // Wait for server response (e.g. from previous request)
                let req = Payload::builder().set_data_utf8("").build();
                let _ = client.request_response(req).await;
            }

            let content = self.env_manager.resolve_string(env_name, &message.content);
            let payload = Payload::builder().set_data_utf8(content.as_str()).build();

            println!("Sending: {}", content);

            match client.request_response(payload).await {
                Ok(Some(response)) => {
                    println!("Received: {:?}", response);
                }
                Ok(None) => {
                    println!("Received: (empty)");
                }
                Err(e) => {
                    return Err(e).context("RSocket request_response failed");
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uri_to_transport_addr_ws() {
        assert_eq!(
            uri_to_transport_addr("ws://localhost:8080/rsocket").unwrap(),
            "ws://localhost:8080/rsocket"
        );
    }

    #[test]
    fn test_uri_to_transport_addr_wss() {
        assert_eq!(
            uri_to_transport_addr("wss://example.com/rsocket").unwrap(),
            "wss://example.com/rsocket"
        );
    }

    #[test]
    fn test_uri_to_transport_addr_rs_converted_to_ws() {
        assert_eq!(
            uri_to_transport_addr("rs://localhost:7878").unwrap(),
            "ws://localhost:7878"
        );
    }

    #[test]
    fn test_uri_to_transport_addr_tcp_converted_to_ws() {
        assert_eq!(
            uri_to_transport_addr("tcp://127.0.0.1:7878").unwrap(),
            "ws://127.0.0.1:7878"
        );
    }

    #[test]
    fn test_uri_to_transport_addr_plain_prefixed_with_ws() {
        assert_eq!(
            uri_to_transport_addr("localhost:8080").unwrap(),
            "ws://localhost:8080"
        );
    }

    #[test]
    fn test_uri_to_transport_addr_trimmed() {
        assert_eq!(
            uri_to_transport_addr("  ws://host:90  ").unwrap(),
            "ws://host:90"
        );
    }

    #[test]
    fn test_uri_to_transport_addr_unsupported_scheme() {
        assert!(uri_to_transport_addr("http://example.com").is_err());
        assert!(uri_to_transport_addr("https://example.com").is_err());
        assert!(uri_to_transport_addr("ftp://host/path").is_err());
    }
}
