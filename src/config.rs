use anyhow::{Context, Result};
use reqwest::ClientBuilder;
use std::path::Path;
use crate::env::{SslConfiguration, CertificateConfig};

#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    pub proxy: Option<ProxyConfig>,
    pub ssl_config: Option<SslConfiguration>,
    pub verify_certificates: bool,
    pub http_version: Option<reqwest::Version>,
}

impl HttpClientConfig {
    pub fn new() -> Self {
        Self {
            proxy: None,
            ssl_config: None,
            verify_certificates: true,
            http_version: None,
        }
    }

    pub fn with_ssl_config(mut self, ssl_config: SslConfiguration) -> Self {
        if let Some(verify) = ssl_config.verify_host_certificate {
            self.verify_certificates = verify;
        }
        self.ssl_config = Some(ssl_config);
        self
    }

    pub fn with_proxy(mut self, proxy: ProxyConfig) -> Self {
        self.proxy = Some(proxy);
        self
    }

    pub fn with_http_version(mut self, version: reqwest::Version) -> Self {
        self.http_version = Some(version);
        self
    }

    pub fn build_client(&self, base_path: impl AsRef<Path>) -> Result<reqwest::Client> {
        let mut builder = ClientBuilder::new();

        // Configure proxy
        if let Some(proxy) = &self.proxy {
            let proxy_url = if let (Some(user), Some(pass)) = (&proxy.username, &proxy.password) {
                format!("http://{}:{}@{}:{}", user, pass, proxy.host, proxy.port)
            } else {
                format!("http://{}:{}", proxy.host, proxy.port)
            };
            builder = builder.proxy(reqwest::Proxy::http(&proxy_url)?);
        }

        // Configure SSL/TLS
        if let Some(ssl_config) = &self.ssl_config {
            if !self.verify_certificates {
                builder = builder.danger_accept_invalid_certs(true);
            }

            // Load client certificate if provided
            if let Some(cert_config) = &ssl_config.client_certificate {
                let cert_path = resolve_cert_path(base_path.as_ref(), cert_config)?;
                let _cert_data = std::fs::read(&cert_path)
                    .with_context(|| format!("Failed to read certificate: {:?}", cert_path))?;

                let key_path = ssl_config.client_certificate_key
                    .as_ref()
                    .map(|k| resolve_cert_path(base_path.as_ref(), k))
                    .transpose()?;

                let _key_data = if let Some(kp) = key_path {
                    Some(std::fs::read(&kp)
                        .with_context(|| format!("Failed to read key: {:?}", kp))?)
                } else {
                    None
                };

                // Note: reqwest doesn't directly support client certificates in the same way
                // This would require using rustls directly, which is more complex
                // For now, we'll skip this and document it as a limitation
            }
        }

        // Configure HTTP version
        // Note: reqwest 0.11 doesn't have direct http_version method
        // HTTP version is negotiated automatically

        builder.build().context("Failed to build HTTP client")
    }
}

fn resolve_cert_path(base: &Path, config: &CertificateConfig) -> Result<std::path::PathBuf> {
    match config {
        CertificateConfig::Path(path) => {
            let path = Path::new(path);
            if path.is_absolute() {
                Ok(path.to_path_buf())
            } else {
                Ok(base.join(path))
            }
        }
        CertificateConfig::Detailed { path, .. } => {
            let path = Path::new(path);
            if path.is_absolute() {
                Ok(path.to_path_buf())
            } else {
                Ok(base.join(path))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::SslConfiguration;

    #[test]
    fn test_http_client_config_new() {
        let config = HttpClientConfig::new();
        assert!(config.proxy.is_none());
        assert!(config.ssl_config.is_none());
        assert!(config.verify_certificates);
        assert!(config.http_version.is_none());
    }

    #[test]
    fn test_http_client_config_with_proxy() {
        let proxy = ProxyConfig {
            host: "proxy.example.com".to_string(),
            port: 8080,
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
        };
        let config = HttpClientConfig::new().with_proxy(proxy.clone());
        assert!(config.proxy.is_some());
        let config_proxy = config.proxy.unwrap();
        assert_eq!(config_proxy.host, proxy.host);
        assert_eq!(config_proxy.port, proxy.port);
    }

    #[test]
    fn test_http_client_config_with_ssl() {
        let ssl_config = SslConfiguration {
            client_certificate: None,
            client_certificate_key: None,
            has_certificate_passphrase: None,
            verify_host_certificate: Some(false),
        };
        let config = HttpClientConfig::new().with_ssl_config(ssl_config);
        assert!(config.ssl_config.is_some());
        assert!(!config.verify_certificates);
    }

    #[test]
    fn test_resolve_cert_path_absolute() {
        let base = Path::new("/tmp");
        let config = CertificateConfig::Path("/absolute/path/cert.pem".to_string());
        let result = resolve_cert_path(base, &config).unwrap();
        assert_eq!(result, Path::new("/absolute/path/cert.pem"));
    }

    #[test]
    fn test_resolve_cert_path_relative() {
        let base = Path::new("/tmp");
        let config = CertificateConfig::Path("cert.pem".to_string());
        let result = resolve_cert_path(base, &config).unwrap();
        assert_eq!(result, Path::new("/tmp/cert.pem"));
    }

    #[test]
    fn test_proxy_config() {
        let proxy = ProxyConfig {
            host: "localhost".to_string(),
            port: 3128,
            username: None,
            password: None,
        };
        assert_eq!(proxy.host, "localhost");
        assert_eq!(proxy.port, 3128);
    }
}
