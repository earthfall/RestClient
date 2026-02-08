use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    #[serde(flatten)]
    pub variables: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ssl_config: Option<SslConfiguration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_certificate: Option<CertificateConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_certificate_key: Option<CertificateConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_certificate_passphrase: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify_host_certificate: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CertificateConfig {
    Path(String),
    Detailed {
        path: String,
        format: Option<String>,
    },
}

#[derive(Clone)]
pub struct EnvironmentManager {
    environments: HashMap<String, Environment>,
    private_env_path: Option<PathBuf>,
    base_path: PathBuf,
}

impl EnvironmentManager {
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            environments: HashMap::new(),
            private_env_path: None,
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    pub fn load_private_env(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if path.exists() {
            let content = fs::read_to_string(path)
                .with_context(|| format!("Failed to read private env file: {:?}", path))?;
            let envs: HashMap<String, Environment> = serde_json::from_str(&content)
                .with_context(|| "Failed to parse private env file")?;
            
            self.environments.extend(envs);
            self.private_env_path = Some(path.to_path_buf());
        }
        Ok(())
    }

    pub fn load_env_file(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if path.exists() {
            let content = fs::read_to_string(path)
                .with_context(|| format!("Failed to read env file: {:?}", path))?;
            let envs: HashMap<String, Environment> = serde_json::from_str(&content)
                .with_context(|| "Failed to parse env file")?;
            
            for (name, env) in envs {
                self.environments.entry(name)
                    .and_modify(|e| {
                        e.variables.extend(env.variables.clone());
                        if env.ssl_config.is_some() {
                            e.ssl_config = env.ssl_config.clone();
                        }
                    })
                    .or_insert(env);
            }
        }
        Ok(())
    }

    pub fn get_environment(&self, name: &str) -> Option<&Environment> {
        self.environments.get(name)
    }

    pub fn resolve_variable(&self, env_name: &str, var_name: &str) -> Option<String> {
        self.environments
            .get(env_name)
            .and_then(|env| env.variables.get(var_name))
            .and_then(|v| match v {
                serde_json::Value::String(s) => Some(s.clone()),
                serde_json::Value::Number(n) => Some(n.to_string()),
                serde_json::Value::Bool(b) => Some(b.to_string()),
                _ => None,
            })
    }

    pub fn resolve_string(&self, env_name: &str, text: &str) -> String {
        let mut result = text.to_string();
        
        // Replace {{variable}} patterns
        let re = regex::Regex::new(r"\{\{([^}]+)\}\}").unwrap();
        result = re.replace_all(&result, |caps: &regex::Captures| {
            let var_name = caps.get(1).unwrap().as_str().trim();
            self.resolve_variable(env_name, var_name)
                .unwrap_or_else(|| caps.get(0).unwrap().as_str().to_string())
        }).to_string();
        
        result
    }

    pub fn get_ssl_config(&self, env_name: &str) -> Option<&SslConfiguration> {
        self.environments
            .get(env_name)
            .and_then(|env| env.ssl_config.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_resolution() {
        let mut manager = EnvironmentManager::new(".");
        let mut env = Environment {
            variables: HashMap::new(),
            ssl_config: None,
        };
        env.variables.insert("API_URL".to_string(), serde_json::Value::String("https://api.example.com".to_string()));
        env.variables.insert("PORT".to_string(), serde_json::Value::Number(8080.into()));
        manager.environments.insert("dev".to_string(), env);

        assert_eq!(manager.resolve_variable("dev", "API_URL"), Some("https://api.example.com".to_string()));
        assert_eq!(manager.resolve_string("dev", "{{API_URL}}/users"), "https://api.example.com/users");
    }

    #[test]
    fn test_resolve_string_with_multiple_variables() {
        let mut manager = EnvironmentManager::new(".");
        let mut env = Environment {
            variables: HashMap::new(),
            ssl_config: None,
        };
        env.variables.insert("BASE_URL".to_string(), serde_json::Value::String("https://api.example.com".to_string()));
        env.variables.insert("VERSION".to_string(), serde_json::Value::String("v1".to_string()));
        manager.environments.insert("dev".to_string(), env);

        let result = manager.resolve_string("dev", "{{BASE_URL}}/{{VERSION}}/users");
        assert_eq!(result, "https://api.example.com/v1/users");
    }

    #[test]
    fn test_resolve_string_with_unknown_variable() {
        let mut manager = EnvironmentManager::new(".");
        let mut env = Environment {
            variables: HashMap::new(),
            ssl_config: None,
        };
        manager.environments.insert("dev".to_string(), env);

        // Unknown variable should remain unchanged
        let result = manager.resolve_string("dev", "{{UNKNOWN_VAR}}/users");
        assert_eq!(result, "{{UNKNOWN_VAR}}/users");
    }

    #[test]
    fn test_resolve_variable_number() {
        let mut manager = EnvironmentManager::new(".");
        let mut env = Environment {
            variables: HashMap::new(),
            ssl_config: None,
        };
        env.variables.insert("PORT".to_string(), serde_json::Value::Number(8080.into()));
        manager.environments.insert("dev".to_string(), env);

        assert_eq!(manager.resolve_variable("dev", "PORT"), Some("8080".to_string()));
    }

    #[test]
    fn test_resolve_variable_bool() {
        let mut manager = EnvironmentManager::new(".");
        let mut env = Environment {
            variables: HashMap::new(),
            ssl_config: None,
        };
        env.variables.insert("DEBUG".to_string(), serde_json::Value::Bool(true));
        manager.environments.insert("dev".to_string(), env);

        assert_eq!(manager.resolve_variable("dev", "DEBUG"), Some("true".to_string()));
    }

    #[test]
    fn test_resolve_variable_nonexistent_env() {
        let manager = EnvironmentManager::new(".");
        assert_eq!(manager.resolve_variable("nonexistent", "VAR"), None);
    }

    #[test]
    fn test_resolve_string_no_variables() {
        let manager = EnvironmentManager::new(".");
        let result = manager.resolve_string("dev", "https://api.example.com/users");
        assert_eq!(result, "https://api.example.com/users");
    }

    #[test]
    fn test_get_ssl_config() {
        let mut manager = EnvironmentManager::new(".");
        let ssl_config = SslConfiguration {
            client_certificate: Some(CertificateConfig::Path("cert.pem".to_string())),
            client_certificate_key: None,
            has_certificate_passphrase: Some(true),
            verify_host_certificate: Some(false),
        };
        let mut env = Environment {
            variables: HashMap::new(),
            ssl_config: Some(ssl_config.clone()),
        };
        manager.environments.insert("dev".to_string(), env);

        let config = manager.get_ssl_config("dev");
        assert!(config.is_some());
        assert_eq!(config.unwrap().verify_host_certificate, Some(false));
    }
}
