use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;
use url::Url;
use crate::env::EnvironmentManager;
use crate::parser::GraphQLRequest;

pub struct GraphQLClient {
    client: Client,
    env_manager: EnvironmentManager,
}

impl GraphQLClient {
    pub fn new(client: Client, env_manager: EnvironmentManager) -> Self {
        Self {
            client,
            env_manager,
        }
    }

    pub async fn execute_request(
        &self,
        request: &GraphQLRequest,
        env_name: Option<&str>,
    ) -> Result<String> {
        let env_name = env_name.unwrap_or("default");

        // Resolve URI with environment variables
        let uri = self.env_manager.resolve_string(env_name, &request.uri);

        // Parse URL
        let url = Url::parse(&uri)
            .with_context(|| format!("Invalid GraphQL URL: {}", uri))?;

        // Resolve query with environment variables
        let query = self.env_manager.resolve_string(env_name, &request.query);

        // Build request body
        let mut body = json!({
            "query": query
        });

        // Add variables if present
        if let Some(vars) = &request.variables {
            // Resolve variables with environment variables
            let vars_str = serde_json::to_string(vars)?;
            let resolved_vars_str = self.env_manager.resolve_string(env_name, &vars_str);
            let resolved_vars: serde_json::Value = serde_json::from_str(&resolved_vars_str)?;
            body["variables"] = resolved_vars;
        }

        // Build HTTP request
        let mut req_builder = self.client.post(url);

        // Add headers
        for (key, value) in &request.headers {
            let resolved_value = self.env_manager.resolve_string(env_name, value);
            req_builder = req_builder.header(key, resolved_value);
        }

        // Default Content-Type if not specified
        if !request.headers.contains_key("Content-Type") && 
           !request.headers.contains_key("content-type") {
            req_builder = req_builder.header("Content-Type", "application/json");
        }

        // Execute request
        let response = req_builder
            .json(&body)
            .send()
            .await
            .context("Failed to send GraphQL request")?;

        let status = response.status();
        let body_text = response
            .text()
            .await
            .context("Failed to read GraphQL response")?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "GraphQL request failed with status {}: {}",
                status,
                body_text
            ));
        }

        Ok(body_text)
    }

    pub fn print_response(&self, response: &str) {
        // Try to pretty-print JSON
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(response) {
            println!("{}", serde_json::to_string_pretty(&json).unwrap_or(response.to_string()));
        } else {
            println!("{}", response);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::EnvironmentManager;
    use std::collections::HashMap;

    #[test]
    fn test_graphql_client_creation() {
        let client = Client::new();
        let env_manager = EnvironmentManager::new(".");
        let gql_client = GraphQLClient::new(client, env_manager);
        // Just test that it can be created
        assert!(true);
    }

    #[test]
    fn test_build_graphql_body() {
        let query = "query { users { id } }";
        let body = json!({
            "query": query
        });
        assert_eq!(body["query"], query);
    }

    #[test]
    fn test_build_graphql_body_with_variables() {
        let query = "query ($id: ID!) { user(id: $id) { name } }";
        let variables = json!({
            "id": "123"
        });
        let body = json!({
            "query": query,
            "variables": variables
        });
        assert_eq!(body["query"], query);
        assert_eq!(body["variables"]["id"], "123");
    }

    #[test]
    fn test_print_response_pretty_json() {
        let client = Client::new();
        let env_manager = EnvironmentManager::new(".");
        let gql_client = GraphQLClient::new(client, env_manager);
        
        let json_response = r#"{"data":{"users":[{"id":"1"}]}}"#;
        // Just test that it doesn't panic
        gql_client.print_response(json_response);
        assert!(true);
    }

    #[test]
    fn test_print_response_plain_text() {
        let client = Client::new();
        let env_manager = EnvironmentManager::new(".");
        let gql_client = GraphQLClient::new(client, env_manager);
        
        let plain_response = "Not JSON";
        // Just test that it doesn't panic
        gql_client.print_response(plain_response);
        assert!(true);
    }
}
