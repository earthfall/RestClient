use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;

pub struct CurlConverter;

impl CurlConverter {
    pub fn curl_to_http(curl_command: &str) -> Result<String> {
        let mut method = "GET".to_string();
        let mut url = String::new();
        let mut headers = HashMap::new();
        let mut body: Option<String> = None;

        // Remove 'curl' prefix and clean up
        let command = curl_command.trim().strip_prefix("curl").unwrap_or(curl_command).trim();

        // Parse URL (first quoted or unquoted string)
        let url_re = Regex::new(r#"(?:^|\s)['"]?([^'"\s]+://[^'"\s]+)['"]?"#)?;
        if let Some(caps) = url_re.captures(command) {
            url = caps.get(1).unwrap().as_str().to_string();
        }

        // Parse method (-X flag)
        let method_re = Regex::new(r#"-X\s+(\w+)"#)?;
        if let Some(caps) = method_re.captures(command) {
            method = caps.get(1).unwrap().as_str().to_uppercase();
        }

        // Parse headers (-H flag)
        let header_re = Regex::new(r#"-H\s+['"]([^'"]+)['"]"#)?;
        for caps in header_re.captures_iter(command) {
            let header = caps.get(1).unwrap().as_str();
            if let Some(colon_pos) = header.find(':') {
                let key = header[..colon_pos].trim().to_string();
                let value = header[colon_pos + 1..].trim().to_string();
                headers.insert(key, value);
            }
        }

        // Parse data (-d or --data flag)
        // Try to match with single quotes, double quotes, or no quotes
        // Handle escaped quotes in JSON
        let data_re = Regex::new(r#"(?:-d|--data)\s+(?:'([^']*(?:\\'[^']*)*)'|"([^"]*(?:\\"[^"]*)*)"|([^\s]+))"#)?;
        if let Some(caps) = data_re.captures(command) {
            let body_str = caps.get(1)
                .map(|m| m.as_str())
                .or_else(|| caps.get(2).map(|m| m.as_str()))
                .or_else(|| caps.get(3).map(|m| m.as_str()))
                .unwrap_or("");
            if !body_str.is_empty() {
                // Unescape the string
                let unescaped = body_str.replace("\\\"", "\"").replace("\\'", "'");
                body = Some(unescaped);
                if method == "GET" {
                    method = "POST".to_string();
                }
            }
        }

        // Build HTTP request format
        let mut result = format!("# Converted from cURL\n");
        result.push_str(&format!("###\n"));
        result.push_str(&format!("{} {}\n", method, url));

        for (key, value) in &headers {
            result.push_str(&format!("{}: {}\n", key, value));
        }

        if let Some(body_content) = body {
            if !headers.is_empty() {
                result.push_str("\n");
            }
            result.push_str(&format!("{}\n", body_content));
        }

        Ok(result)
    }

    pub fn http_to_curl(request: &str) -> Result<String> {
        let lines: Vec<&str> = request.lines().collect();
        let mut method = "GET";
        let mut url = String::new();
        let mut headers = Vec::new();
        let mut body: Option<String> = None;
        let mut in_body = false;
        let mut body_lines = Vec::new();

        for line in lines {
            let line = line.trim();
            
            if line.is_empty() {
                in_body = true;
                continue;
            }

            if line.starts_with("###") || line.starts_with("#") || line.starts_with("//") {
                continue;
            }

            if !in_body {
                if line.contains("://") {
                    // This is the URL line
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        method = parts[0];
                        url = parts[1].to_string();
                    } else if parts.len() == 1 {
                        url = parts[0].to_string();
                    }
                } else if line.contains(':') && !line.starts_with("http") && !line.starts_with("ws") {
                    // This is a header (but not a URL)
                    headers.push(line.to_string());
                }
            } else {
                if !line.is_empty() || !body_lines.is_empty() {
                    body_lines.push(line);
                }
            }
        }

        if !body_lines.is_empty() {
            body = Some(body_lines.join("\n"));
        }

        // Build cURL command
        let mut curl = format!("curl '{}'", url);

        if method != "GET" {
            curl = format!("curl -X {} '{}'", method, url);
        }

        for header in headers {
            curl.push_str(&format!(" -H '{}'", header));
        }

        if let Some(body_content) = body {
            curl.push_str(&format!(" -d '{}'", body_content.replace("'", "'\\''")));
        }

        Ok(curl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curl_to_http() {
        let curl = "curl 'https://httpbin.org/' -H 'Connection: keep-alive' -H 'Accept: text/html'";
        let http = CurlConverter::curl_to_http(curl).unwrap();
        assert!(http.contains("GET"));
        assert!(http.contains("https://httpbin.org/"));
        assert!(http.contains("Connection: keep-alive"));
    }

    #[test]
    fn test_curl_to_http_with_post() {
        let curl = "curl -X POST 'https://httpbin.org/post' -H 'Content-Type: application/json' -d '{\"name\":\"test\"}'";
        let http = CurlConverter::curl_to_http(curl).unwrap();
        eprintln!("Generated HTTP:\n{}", http);
        assert!(http.contains("POST"), "HTTP should contain POST method");
        assert!(http.contains("https://httpbin.org/post"), "HTTP should contain URL");
        assert!(http.contains("Content-Type: application/json"), "HTTP should contain Content-Type header");
        assert!(http.contains("name") || http.contains("test"), "HTTP should contain body with 'name' or 'test'");
    }

    #[test]
    fn test_curl_to_http_with_multiple_headers() {
        let curl = "curl 'https://api.example.com/users' -H 'Accept: application/json' -H 'Authorization: Bearer token123'";
        let http = CurlConverter::curl_to_http(curl).unwrap();
        assert!(http.contains("Accept: application/json"));
        assert!(http.contains("Authorization: Bearer token123"));
    }

    #[test]
    fn test_http_to_curl_get() {
        let http = r###"
### Get Users
GET https://api.example.com/users
Accept: application/json
"###;
        let curl = CurlConverter::http_to_curl(http).unwrap();
        assert!(curl.contains("curl"));
        assert!(curl.contains("https://api.example.com/users"));
        assert!(curl.contains("Accept: application/json"));
    }

    #[test]
    fn test_http_to_curl_post() {
        let http = r###"
### Create User
POST https://api.example.com/users
Content-Type: application/json

{
  "name": "John"
}
"###;
        let curl = CurlConverter::http_to_curl(http).unwrap();
        eprintln!("Generated cURL:\n{}", curl);
        assert!(curl.contains("-X POST") || curl.contains("POST"), "cURL should contain POST method");
        assert!(curl.contains("https://api.example.com/users"), "cURL should contain URL");
        assert!(curl.contains("-d"), "cURL should contain -d flag for body");
    }

    #[test]
    fn test_http_to_curl_with_headers() {
        let http = r###"
GET https://api.example.com/users
Authorization: Bearer token123
Accept: application/json
"###;
        let curl = CurlConverter::http_to_curl(http).unwrap();
        assert!(curl.contains("Authorization: Bearer token123"));
        assert!(curl.contains("Accept: application/json"));
    }

    #[test]
    fn test_curl_with_quotes() {
        let curl = r#"curl "https://httpbin.org/get""#;
        let http = CurlConverter::curl_to_http(curl).unwrap();
        assert!(http.contains("https://httpbin.org/get"));
    }

    #[test]
    fn test_curl_without_quotes() {
        let curl = "curl https://httpbin.org/get";
        let http = CurlConverter::curl_to_http(curl).unwrap();
        assert!(http.contains("https://httpbin.org/get"));
    }
}
