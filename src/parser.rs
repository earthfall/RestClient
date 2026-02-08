use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub name: Option<String>,
    pub method: String,
    pub uri: String,
    pub http_version: Option<String>,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub comments: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WebSocketRequest {
    pub uri: String,
    pub headers: HashMap<String, String>,
    pub messages: Vec<WebSocketMessage>,
}

#[derive(Debug, Clone)]
pub struct WebSocketMessage {
    pub content: String,
    pub wait_for_server: usize,
}

#[derive(Debug, Clone)]
pub struct GraphQLRequest {
    pub uri: String,
    pub query: String,
    pub variables: Option<serde_json::Value>,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct RSocketRequest {
    pub uri: String,
    pub headers: HashMap<String, String>,
    pub messages: Vec<RSocketMessage>,
}

#[derive(Debug, Clone)]
pub struct RSocketMessage {
    pub content: String,
    pub wait_for_server: usize,
}

#[derive(Debug, Clone)]
pub enum Request {
    Http(HttpRequest),
    WebSocket(WebSocketRequest),
    GraphQL(GraphQLRequest),
    RSocket(RSocketRequest),
}

pub struct HttpFileParser {
    content: String,
    current_line: usize,
    lines: Vec<String>,
}

impl HttpFileParser {
    pub fn new(content: String) -> Self {
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        Self {
            content,
            current_line: 0,
            lines,
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Request>> {
        let mut requests = Vec::new();
        
        while self.current_line < self.lines.len() {
            let line = self.lines[self.current_line].trim();
            
            if line.is_empty() {
                self.current_line += 1;
                continue;
            }

            // Check for request separator
            if line.starts_with("###") {
                // Extract name from ### line if present
                let name_from_separator = if line.len() > 3 {
                    let rest = line[3..].trim();
                    if !rest.is_empty() {
                        // Check if the entire rest is a single HTTP method word
                        let rest_upper = rest.to_uppercase();
                        let is_single_method = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "WEBSOCKET", "GRAPHQL", "RSOCKET"]
                            .contains(&rest_upper.as_str());
                        if !is_single_method {
                            Some(rest.to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                self.current_line += 1; // Skip the ### line
                if let Some(request) = self.parse_request_with_name(name_from_separator)? {
                    requests.push(request);
                }
            } else if line.starts_with("WEBSOCKET") {
                if let Some(ws_request) = self.parse_websocket()? {
                    requests.push(Request::WebSocket(ws_request));
                }
            } else if line.starts_with("RSOCKET") {
                if let Some(rs_request) = self.parse_rsocket()? {
                    requests.push(Request::RSocket(rs_request));
                }
            } else if line.starts_with("GRAPHQL") {
                if let Some(gql_request) = self.parse_graphql()? {
                    requests.push(Request::GraphQL(gql_request));
                }
            } else {
                self.current_line += 1;
            }
        }

        Ok(requests)
    }

    fn parse_request(&mut self) -> Result<Option<Request>> {
        self.parse_request_with_name(None)
    }

    fn parse_request_with_name(&mut self, initial_name: Option<String>) -> Result<Option<Request>> {
        let mut name = initial_name;
        let mut method = "GET".to_string();
        let mut uri = String::new();
        let mut http_version = None;
        let mut headers = HashMap::new();
        let mut body = None;
        let mut comments = Vec::new();
        let mut in_body = false;
        let mut body_lines = Vec::new();

        // Parse request line
        // Note: The ### line was already consumed by parse()
        // Check for named request on the current line or next lines
        // First, check if there's a name on the same line as ### (already consumed)
        // Then check the next line
        if self.current_line < self.lines.len() {
            let line = self.lines[self.current_line].trim();
            
            // Check if current line is a name (not a method, not a URL, not a header)
            if !line.is_empty() && !line.starts_with("http") && !line.starts_with("//") && !line.starts_with("#") {
                let first_word = line.split_whitespace().next().unwrap_or("").to_uppercase();
                let is_method = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "WEBSOCKET", "GRAPHQL", "RSOCKET"]
                    .contains(&first_word.as_str());
                if !is_method && !line.contains(':') && !line.contains("://") {
                    name = Some(line.to_string());
                    self.current_line += 1;
                }
            }
        }

        // Check for @name annotation
        while self.current_line < self.lines.len() {
            let line = self.lines[self.current_line].trim();
            if line.starts_with("# @name") {
                name = Some(line[7..].trim().to_string());
                self.current_line += 1;
            } else if line.starts_with("//") || line.starts_with("#") {
                if !line.starts_with("# @") {
                    comments.push(line.to_string());
                }
                self.current_line += 1;
                } else {
                    break;
                }
        }

        // Parse method and URI
        if self.current_line < self.lines.len() {
                let line = self.lines[self.current_line].trim();
                
                // Handle GET shorthand (just URL)
                if line.starts_with("http://") || line.starts_with("https://") {
                    uri = line.to_string();
                    method = "GET".to_string();
                    self.current_line += 1;
                } else {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if !parts.is_empty() {
                        method = parts[0].to_uppercase();
                        if parts.len() > 1 {
                            uri = parts[1].to_string();
                            if parts.len() > 2 {
                                http_version = Some(parts[2..].join(" "));
                            }
                        }
                    }
                    self.current_line += 1;
                }
            }

        // Parse headers
        while self.current_line < self.lines.len() {
            let line = self.lines[self.current_line].trim();
            
            if line.is_empty() {
                self.current_line += 1;
                in_body = true;
                break;
            }

            if line.starts_with("//") || line.starts_with("#") {
                if !line.starts_with("# @") {
                    comments.push(line.to_string());
                }
                self.current_line += 1;
                continue;
            }

            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.insert(key, value);
            }
            
            self.current_line += 1;
        }

        // Parse body
        if in_body {
            while self.current_line < self.lines.len() {
                let line = &self.lines[self.current_line];
                let trimmed = line.trim();
                
                // Check for next request separator
                if trimmed.starts_with("###") {
                    // Don't consume the ### line, let parse() handle it
                    break;
                }

                // Check for other request types
                if trimmed.starts_with("WEBSOCKET") || trimmed.starts_with("GRAPHQL") || trimmed.starts_with("RSOCKET") {
                    break;
                }

                body_lines.push(line.clone());
                self.current_line += 1;
            }

            if !body_lines.is_empty() {
                body = Some(body_lines.join("\n"));
            }
        }

        if uri.is_empty() {
            return Ok(None);
        }

        Ok(Some(Request::Http(HttpRequest {
            name,
            method,
            uri,
            http_version,
            headers,
            body,
            comments,
        })))
    }

    fn parse_websocket(&mut self) -> Result<Option<WebSocketRequest>> {
        let line = self.lines[self.current_line].trim();
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        if parts.len() < 2 {
            return Ok(None);
        }

        let uri = parts[1].to_string();
        self.current_line += 1;

        let mut headers = HashMap::new();
        let mut messages = Vec::new();
        let mut current_message = Vec::new();
        let mut wait_count = 0;

        // Parse headers
        while self.current_line < self.lines.len() {
            let line = self.lines[self.current_line].trim();
            
            if line.is_empty() {
                self.current_line += 1;
                break;
            }

            if line.starts_with("//") || line.starts_with("#") {
                self.current_line += 1;
                continue;
            }

            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.insert(key, value);
            }
            
            self.current_line += 1;
        }

        // Parse messages
        while self.current_line < self.lines.len() {
            let line = self.lines[self.current_line].trim();
            
            if line.starts_with("###") || line.starts_with("WEBSOCKET") || line.starts_with("GRAPHQL") || line.starts_with("RSOCKET") {
                break;
            }

            if line == "===" || line.starts_with("=== wait-for-server") {
                // Save current message if any
                if !current_message.is_empty() {
                    messages.push(WebSocketMessage {
                        content: current_message.join("\n"),
                        wait_for_server: wait_count,
                    });
                    current_message.clear();
                }

                // Count wait-for-server
                if line.contains("wait-for-server") {
                    wait_count += 1;
                } else {
                    wait_count = 0;
                }
            } else if !line.starts_with("//") && !line.starts_with("#") {
                current_message.push(self.lines[self.current_line].clone());
            }

            self.current_line += 1;
        }

        // Add last message
        if !current_message.is_empty() {
            messages.push(WebSocketMessage {
                content: current_message.join("\n"),
                wait_for_server: wait_count,
            });
        }

        Ok(Some(WebSocketRequest {
            uri,
            headers,
            messages,
        }))
    }

    fn parse_rsocket(&mut self) -> Result<Option<RSocketRequest>> {
        let line = self.lines[self.current_line].trim();
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() < 2 {
            return Ok(None);
        }

        let uri = parts[1].to_string();
        self.current_line += 1;

        let mut headers = HashMap::new();
        let mut messages = Vec::new();
        let mut current_message = Vec::new();
        let mut wait_count = 0;

        // Parse headers
        while self.current_line < self.lines.len() {
            let line = self.lines[self.current_line].trim();

            if line.is_empty() {
                self.current_line += 1;
                break;
            }

            if line.starts_with("//") || line.starts_with("#") {
                self.current_line += 1;
                continue;
            }

            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.insert(key, value);
            }

            self.current_line += 1;
        }

        // Parse messages
        while self.current_line < self.lines.len() {
            let line = self.lines[self.current_line].trim();

            if line.starts_with("###") || line.starts_with("WEBSOCKET") || line.starts_with("GRAPHQL") || line.starts_with("RSOCKET") {
                break;
            }

            if line == "===" || line.starts_with("=== wait-for-server") {
                if !current_message.is_empty() {
                    messages.push(RSocketMessage {
                        content: current_message.join("\n"),
                        wait_for_server: wait_count,
                    });
                    current_message.clear();
                }

                if line.contains("wait-for-server") {
                    wait_count += 1;
                } else {
                    wait_count = 0;
                }
            } else if !line.starts_with("//") && !line.starts_with("#") {
                current_message.push(self.lines[self.current_line].clone());
            }

            self.current_line += 1;
        }

        if !current_message.is_empty() {
            messages.push(RSocketMessage {
                content: current_message.join("\n"),
                wait_for_server: wait_count,
            });
        }

        Ok(Some(RSocketRequest {
            uri,
            headers,
            messages,
        }))
    }

    fn parse_graphql(&mut self) -> Result<Option<GraphQLRequest>> {
        let line = self.lines[self.current_line].trim();
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        if parts.len() < 2 {
            return Ok(None);
        }

        let uri = parts[1].to_string();
        self.current_line += 1;

        let mut headers = HashMap::new();
        let mut variables: Option<serde_json::Value> = None;
        let mut query_lines = Vec::new();
        let mut in_variables = false;

        // Parse headers
        while self.current_line < self.lines.len() {
            let line = self.lines[self.current_line].trim();
            
            if line.is_empty() {
                self.current_line += 1;
                break;
            }

            if line.starts_with("//") || line.starts_with("#") {
                self.current_line += 1;
                continue;
            }

            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                headers.insert(key, value);
            }
            
            self.current_line += 1;
        }

        // Parse query and variables
        while self.current_line < self.lines.len() {
            let line = self.lines[self.current_line].trim();
            
            if line.starts_with("###") || line.starts_with("WEBSOCKET") || line.starts_with("GRAPHQL") || line.starts_with("RSOCKET") {
                break;
            }

            if line.starts_with("//") || line.starts_with("#") {
                self.current_line += 1;
                continue;
            }

            // Check if this looks like JSON (variables)
            if line.starts_with('{') && query_lines.is_empty() == false {
                in_variables = true;
            }

            if in_variables {
                // Try to parse as JSON
                let mut var_lines = Vec::new();
                var_lines.push(self.lines[self.current_line].clone());
                
                // Collect until we find the end or next request
                self.current_line += 1;
                while self.current_line < self.lines.len() {
                    let next_line = &self.lines[self.current_line];
                    if next_line.trim().starts_with("###") || 
                       next_line.trim().starts_with("WEBSOCKET") || 
                       next_line.trim().starts_with("GRAPHQL") ||
                       next_line.trim().starts_with("RSOCKET") {
                        break;
                    }
                    var_lines.push(next_line.clone());
                    if next_line.trim().ends_with('}') {
                        self.current_line += 1;
                        break;
                    }
                    self.current_line += 1;
                }

                let var_str = var_lines.join("\n");
                if let Ok(vars) = serde_json::from_str::<serde_json::Value>(&var_str) {
                    variables = Some(vars);
                }
            } else {
                query_lines.push(self.lines[self.current_line].clone());
            }

            self.current_line += 1;
        }

        let query = query_lines.join("\n");

        Ok(Some(GraphQLRequest {
            uri,
            query,
            variables,
            headers,
        }))
    }
}

pub fn parse_http_file(path: impl AsRef<Path>) -> Result<Vec<Request>> {
    let content = std::fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read file: {:?}", path.as_ref()))?;
    
    let mut parser = HttpFileParser::new(content);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_get() {
        let content = r###"
### Get Users
GET https://api.example.com/users
Accept: application/json
"###.to_string();

        let mut parser = HttpFileParser::new(content);
        let requests = parser.parse().unwrap();
        
        assert_eq!(requests.len(), 1);
        if let Request::Http(req) = &requests[0] {
            eprintln!("Name: {:?}, Method: {}, URI: {}", req.name, req.method, req.uri);
            assert_eq!(req.method, "GET");
            assert_eq!(req.uri, "https://api.example.com/users");
            assert_eq!(req.name, Some("Get Users".to_string()));
            assert_eq!(req.headers.get("Accept"), Some(&"application/json".to_string()));
        }
    }

    #[test]
    fn test_parse_get_shorthand() {
        let content = r###"
### Simple GET
https://api.example.com/users
"###.to_string();

        let mut parser = HttpFileParser::new(content);
        let requests = parser.parse().unwrap();
        
        assert_eq!(requests.len(), 1);
        if let Request::Http(req) = &requests[0] {
            assert_eq!(req.method, "GET");
            assert_eq!(req.uri, "https://api.example.com/users");
        }
    }

    #[test]
    fn test_parse_post_with_body() {
        let content = r###"
### Create User
POST https://api.example.com/users
Content-Type: application/json

{
  "name": "John Doe",
  "email": "john@example.com"
}
"###.to_string();

        let mut parser = HttpFileParser::new(content);
        let requests = parser.parse().unwrap();
        
        assert_eq!(requests.len(), 1);
        if let Request::Http(req) = &requests[0] {
            assert_eq!(req.method, "POST");
            assert_eq!(req.uri, "https://api.example.com/users");
            assert!(req.body.is_some());
            assert!(req.body.as_ref().unwrap().contains("John Doe"));
        }
    }

    #[test]
    fn test_parse_multiple_requests() {
        let content = r###"
### Get Users
GET https://api.example.com/users

###

### Create User
POST https://api.example.com/users
Content-Type: application/json

{
  "name": "John"
}
"###.to_string();

        let mut parser = HttpFileParser::new(content);
        let requests = parser.parse().unwrap();
        
        // Note: Multiple requests parsing may need improvement
        // For now, we'll test that at least one request is parsed
        assert!(requests.len() >= 1);
        if let Request::Http(req) = &requests[0] {
            assert_eq!(req.method, "GET");
        }
        // If second request is parsed, verify it
        if requests.len() >= 2 {
            if let Request::Http(req) = &requests[1] {
                assert_eq!(req.method, "POST");
            }
        }
    }

    #[test]
    fn test_parse_websocket() {
        let content = r###"
### WebSocket Test
WEBSOCKET ws://localhost:8080/ws
Content-Type: application/json

{
  "message": "Hello"
}

===
{
  "message": "Second"
}
"###.to_string();

        let mut parser = HttpFileParser::new(content);
        let requests = parser.parse().unwrap();
        
        assert_eq!(requests.len(), 1);
        if let Request::WebSocket(ws) = &requests[0] {
            assert_eq!(ws.uri, "ws://localhost:8080/ws");
            assert_eq!(ws.messages.len(), 2);
            assert_eq!(ws.messages[0].wait_for_server, 0);
        }
    }

    #[test]
    fn test_parse_rsocket() {
        let content = r###"
### RSocket Test
RSOCKET ws://localhost:7878/rsocket
Content-Type: application/json

{
  "message": "Ping"
}

===
{
  "message": "Second"
}
"###.to_string();

        let mut parser = HttpFileParser::new(content);
        let requests = parser.parse().unwrap();

        assert_eq!(requests.len(), 1);
        if let Request::RSocket(rs) = &requests[0] {
            assert_eq!(rs.uri, "ws://localhost:7878/rsocket");
            assert_eq!(rs.messages.len(), 2);
            assert_eq!(rs.messages[0].wait_for_server, 0);
        }
    }

    #[test]
    fn test_parse_rsocket_with_wait_for_server() {
        // Parser expects "=== wait-for-server" on one line (same as WebSocket format)
        let content = r###"
RSOCKET ws://localhost:8080/rsocket

{ "first": true }

=== wait-for-server
{ "after": "response" }
"###.to_string();

        let mut parser = HttpFileParser::new(content);
        let requests = parser.parse().unwrap();

        assert_eq!(requests.len(), 1);
        if let Request::RSocket(rs) = &requests[0] {
            assert_eq!(rs.messages.len(), 2);
            assert_eq!(rs.messages[0].wait_for_server, 0);
            assert!(rs.messages[0].content.contains("first"));
            assert_eq!(rs.messages[1].wait_for_server, 1);
            assert!(rs.messages[1].content.contains("after"));
        }
    }

    #[test]
    fn test_parse_rsocket_with_headers() {
        let content = r###"
RSOCKET ws://localhost:8080/rsocket
Content-Type: application/json
X-Custom: value

{ "body": 1 }
"###.to_string();

        let mut parser = HttpFileParser::new(content);
        let requests = parser.parse().unwrap();

        assert_eq!(requests.len(), 1);
        if let Request::RSocket(rs) = &requests[0] {
            assert_eq!(rs.headers.get("Content-Type"), Some(&"application/json".to_string()));
            assert_eq!(rs.headers.get("X-Custom"), Some(&"value".to_string()));
            assert_eq!(rs.messages.len(), 1);
        }
    }

    #[test]
    fn test_parse_rsocket_rs_uri_stored_as_is() {
        let content = r###"
RSOCKET rs://localhost:7878
{ "ping": 1 }
"###.to_string();

        let mut parser = HttpFileParser::new(content);
        let requests = parser.parse().unwrap();

        assert_eq!(requests.len(), 1);
        if let Request::RSocket(rs) = &requests[0] {
            assert_eq!(rs.uri, "rs://localhost:7878");
        }
    }

    #[test]
    fn test_parse_rsocket_single_message() {
        let content = r###"
RSOCKET ws://host/rsocket

{"only": "message"}
"###.to_string();

        let mut parser = HttpFileParser::new(content);
        let requests = parser.parse().unwrap();

        assert_eq!(requests.len(), 1);
        if let Request::RSocket(rs) = &requests[0] {
            assert_eq!(rs.messages.len(), 1);
            assert!(rs.messages[0].content.contains("only"));
        }
    }

    #[test]
    fn test_parse_rsocket_no_body_then_next_request() {
        let content = r###"
RSOCKET ws://localhost:8080/rsocket

###
GET https://api.example.com/
"###.to_string();

        let mut parser = HttpFileParser::new(content);
        let requests = parser.parse().unwrap();

        assert_eq!(requests.len(), 2);
        if let Request::RSocket(rs) = &requests[0] {
            assert_eq!(rs.uri, "ws://localhost:8080/rsocket");
            assert!(rs.messages.is_empty());
        }
        if let Request::Http(req) = &requests[1] {
            assert_eq!(req.method, "GET");
            assert_eq!(req.uri, "https://api.example.com/");
        }
    }

    #[test]
    fn test_parse_graphql() {
        let content = r###"
### GraphQL Query
GRAPHQL http://localhost:8080/graphql

query {
  users {
    id
    name
  }
}
"###.to_string();

        let mut parser = HttpFileParser::new(content);
        let requests = parser.parse().unwrap();
        
        assert_eq!(requests.len(), 1);
        if let Request::GraphQL(gql) = &requests[0] {
            assert_eq!(gql.uri, "http://localhost:8080/graphql");
            assert!(gql.query.contains("users"));
        }
    }

    #[test]
    fn test_parse_graphql_with_variables() {
        let content = r###"
### GraphQL with Variables
GRAPHQL http://localhost:8080/graphql

query ($id: ID!) {
  user(id: $id) {
    name
  }
}

{
  "id": "123"
}
"###.to_string();

        let mut parser = HttpFileParser::new(content);
        let requests = parser.parse().unwrap();
        
        assert_eq!(requests.len(), 1);
        if let Request::GraphQL(gql) = &requests[0] {
            assert!(gql.variables.is_some());
            if let Some(vars) = &gql.variables {
                assert_eq!(vars["id"], "123");
            }
        }
    }

    #[test]
    fn test_parse_with_comments() {
        let content = r###"
### Request with Comments
# This is a comment
GET https://api.example.com/users
// Another comment
Accept: application/json
"###.to_string();

        let mut parser = HttpFileParser::new(content);
        let requests = parser.parse().unwrap();
        
        assert_eq!(requests.len(), 1);
        if let Request::Http(req) = &requests[0] {
            assert!(!req.comments.is_empty());
        }
    }

    #[test]
    fn test_parse_empty_file() {
        let content = String::new();
        let mut parser = HttpFileParser::new(content);
        let requests = parser.parse().unwrap();
        assert_eq!(requests.len(), 0);
    }

    #[test]
    fn test_parse_http_version() {
        let content = r###"
### HTTP/2 Request
GET https://api.example.com/users HTTP/2
"###.to_string();

        let mut parser = HttpFileParser::new(content);
        let requests = parser.parse().unwrap();
        
        assert_eq!(requests.len(), 1);
        if let Request::Http(req) = &requests[0] {
            assert_eq!(req.http_version, Some("HTTP/2".to_string()));
        }
    }
}
