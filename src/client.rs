use anyhow::{Context, Result};
use reqwest::{Client, Method};
use std::collections::HashMap;
use std::time::Duration;
use url::Url;
use crate::config::HttpClientConfig;
use crate::env::EnvironmentManager;
use crate::parser::HttpRequest;

#[derive(Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub content_type: Option<String>,
}

pub struct HttpClient {
    client: Client,
    config: HttpClientConfig,
    env_manager: EnvironmentManager,
    base_path: std::path::PathBuf,
}

impl HttpClient {
    pub fn new(
        config: HttpClientConfig,
        env_manager: EnvironmentManager,
        base_path: impl AsRef<std::path::Path>,
    ) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        let client = config.build_client(&base_path)?;

        Ok(Self {
            client,
            config,
            env_manager,
            base_path,
        })
    }

    pub async fn execute_request(
        &self,
        request: &HttpRequest,
        env_name: Option<&str>,
    ) -> Result<HttpResponse> {
        let env_name = env_name.unwrap_or("default");

        // Resolve URI with environment variables
        let uri = self.env_manager.resolve_string(env_name, &request.uri);

        // Parse URL
        let url = Url::parse(&uri)
            .with_context(|| format!("Invalid URL: {}", uri))?;

        // Determine HTTP method
        let method = Method::from_bytes(request.method.as_bytes())
            .with_context(|| format!("Invalid HTTP method: {}", request.method))?;

        // Build request
        let mut req_builder = self.client.request(method, url);

        // Add headers
        for (key, value) in &request.headers {
            let resolved_value = self.env_manager.resolve_string(env_name, value);
            req_builder = req_builder.header(key, resolved_value);
        }

        // Add body
        if let Some(body) = &request.body {
            let resolved_body = self.env_manager.resolve_string(env_name, body);
            
            // Check content type
            let content_type = request.headers
                .get("Content-Type")
                .or_else(|| request.headers.get("content-type"))
                .map(|s| s.to_lowercase());

            match content_type.as_deref() {
                Some("application/json") => {
                    req_builder = req_builder.json(&serde_json::from_str::<serde_json::Value>(&resolved_body)?);
                }
                Some("application/x-www-form-urlencoded") => {
                    // Parse form data
                    let form_data: HashMap<String, String> = resolved_body
                        .split('&')
                        .filter_map(|pair| {
                            let mut parts = pair.splitn(2, '=');
                            let key = parts.next()?.to_string();
                            let value = parts.next().unwrap_or("").to_string();
                            Some((key, value))
                        })
                        .collect();
                    req_builder = req_builder.form(&form_data);
                }
                Some(ct) if ct.starts_with("multipart/form-data") => {
                    // Handle multipart form data
                    // This is simplified - full implementation would parse the body properly
                    req_builder = req_builder.body(resolved_body);
                }
                _ => {
                    req_builder = req_builder.body(resolved_body);
                }
            }
        }

        // Execute request
        let response = req_builder
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .context("Failed to send HTTP request")?;

        let status = response.status().as_u16();
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .map(|(k, v)| {
                (k.to_string(), v.to_str().unwrap_or("").to_string())
            })
            .collect();

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let body = response
            .text()
            .await
            .context("Failed to read response body")?;

        Ok(HttpResponse {
            status,
            headers,
            body,
            content_type,
        })
    }

    pub fn print_response(&self, response: &HttpResponse) {
        println!("HTTP/1.1 {}", response.status);
        for (key, value) in &response.headers {
            println!("{}: {}", key, value);
        }
        println!();
        println!("{}", response.body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HttpClientConfig;
    use std::collections::HashMap;

    #[test]
    fn test_http_response_creation() {
        let response = HttpResponse {
            status: 200,
            headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".to_string(), "application/json".to_string());
                h
            },
            body: r#"{"message": "success"}"#.to_string(),
            content_type: Some("application/json".to_string()),
        };

        assert_eq!(response.status, 200);
        assert_eq!(response.headers.get("Content-Type"), Some(&"application/json".to_string()));
        assert!(response.body.contains("success"));
    }

    #[tokio::test]
    async fn test_http_client_creation() {
        let config = HttpClientConfig::new();
        let env_manager = EnvironmentManager::new(".");
        let client = HttpClient::new(config, env_manager, ".");
        assert!(client.is_ok());
    }

    #[test]
    fn test_parse_form_data() {
        let form_data = "name=John+Doe&email=john%40example.com";
        let parsed: HashMap<String, String> = form_data
            .split('&')
            .filter_map(|pair| {
                let mut parts = pair.splitn(2, '=');
                let key = parts.next()?.to_string();
                let value = parts.next().unwrap_or("").to_string();
                Some((key, value))
            })
            .collect();

        assert_eq!(parsed.get("name"), Some(&"John+Doe".to_string()));
        assert_eq!(parsed.get("email"), Some(&"john%40example.com".to_string()));
    }
}
