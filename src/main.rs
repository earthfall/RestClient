use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use http_client::{
    HttpClientConfig, CurlConverter, EnvironmentManager, GraphQLClient, HttpClient,
    HttpRequest, Request, WebSocketClient, WebSocketRequest, GraphQLRequest,
    RSocketClient, RSocketRequest,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "http-client")]
#[command(about = "HTTP Client implementation in Rust, compatible with IntelliJ IDEA HTTP Client format")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute HTTP requests from a .http or .rest file
    Run {
        /// Path to the .http or .rest file
        file: PathBuf,
        /// Environment name to use
        #[arg(short, long)]
        env: Option<String>,
        /// Path to environment file
        #[arg(short = 'e', long = "env-file")]
        env_file: Option<PathBuf>,
        /// Path to private environment file
        #[arg(short = 'p', long = "private-env-file")]
        private_env_file: Option<PathBuf>,
    },
    /// Convert cURL command to HTTP request format
    Convert {
        /// cURL command to convert
        curl: String,
    },
    /// Convert HTTP request format to cURL command
    ToCurl {
        /// Path to the .http file
        file: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            file,
            env,
            env_file,
            private_env_file,
        } => {
            run_requests(file, env, env_file, private_env_file).await?;
        }
        Commands::Convert { curl } => {
            let http = CurlConverter::curl_to_http(&curl)
                .context("Failed to convert cURL command")?;
            println!("{}", http);
        }
        Commands::ToCurl { file } => {
            let content = std::fs::read_to_string(&file)
                .with_context(|| format!("Failed to read file: {:?}", file))?;
            let curl = CurlConverter::http_to_curl(&content)
                .context("Failed to convert HTTP request to cURL")?;
            println!("{}", curl);
        }
    }

    Ok(())
}

async fn run_requests(
    file: PathBuf,
    env_name: Option<String>,
    env_file: Option<PathBuf>,
    private_env_file: Option<PathBuf>,
) -> Result<()> {
    // Load environment files
    let base_path = file.parent().unwrap_or(std::path::Path::new("."));
    let mut env_manager = EnvironmentManager::new(base_path);

    // Load private environment file first (highest priority)
    if let Some(ref path) = private_env_file {
        env_manager.load_private_env(path)?;
    } else {
        // Try default private env file
        let default_private = base_path.join("http-client.private.env.json");
        if default_private.exists() {
            env_manager.load_private_env(&default_private)?;
        }
    }

    // Load public environment file
    if let Some(ref path) = env_file {
        env_manager.load_env_file(path)?;
    } else {
        // Try default env file
        let default_env = base_path.join("http-client.env.json");
        if default_env.exists() {
            env_manager.load_env_file(&default_env)?;
        }
    }

    // Parse HTTP file
    let requests = http_client::parse_http_file(&file)
        .with_context(|| format!("Failed to parse file: {:?}", file))?;

    if requests.is_empty() {
        println!("No requests found in file");
        return Ok(());
    }

    // Build client config
    let mut client_config = HttpClientConfig::new();

    // Apply SSL config from environment if available
    let env_name_str = env_name.as_deref().unwrap_or("default");
    if let Some(ssl_config) = env_manager.get_ssl_config(env_name_str) {
        client_config = client_config.with_ssl_config(ssl_config.clone());
    }

    // Create HTTP client
    let http_client = HttpClient::new(client_config.clone(), env_manager.clone(), base_path)?;
    let ws_client = WebSocketClient::new(env_manager.clone());
    let rsocket_client = RSocketClient::new(env_manager.clone());
    let graphql_client = GraphQLClient::new(
        client_config.build_client(base_path)?,
        env_manager.clone(),
    );

    // Execute each request
    for (idx, request) in requests.iter().enumerate() {
        if idx > 0 {
            println!("\n{}\n", "=".repeat(80));
        }

        match request {
            Request::Http(http_req) => {
                if let Some(name) = &http_req.name {
                    println!("### {}\n", name);
                }
                execute_http_request(&http_client, http_req, env_name.as_deref()).await?;
            }
            Request::WebSocket(ws_req) => {
                println!("### WebSocket Request\n");
                execute_websocket_request(&ws_client, ws_req, env_name.as_deref()).await?;
            }
            Request::RSocket(rs_req) => {
                println!("### RSocket Request\n");
                execute_rsocket_request(&rsocket_client, rs_req, env_name.as_deref()).await?;
            }
            Request::GraphQL(gql_req) => {
                println!("### GraphQL Request\n");
                execute_graphql_request(&graphql_client, gql_req, env_name.as_deref()).await?;
            }
        }
    }

    Ok(())
}

async fn execute_http_request(
    client: &HttpClient,
    request: &HttpRequest,
    env_name: Option<&str>,
) -> Result<()> {
    println!("{} {}", request.method, request.uri);
    if !request.headers.is_empty() {
        println!("Headers:");
        for (key, value) in &request.headers {
            println!("  {}: {}", key, value);
        }
    }
    if let Some(body) = &request.body {
        println!("Body:\n{}", body);
    }
    println!();

    let response = client
        .execute_request(request, env_name)
        .await
        .context("Failed to execute HTTP request")?;

    client.print_response(&response);
    Ok(())
}

async fn execute_websocket_request(
    client: &WebSocketClient,
    request: &WebSocketRequest,
    env_name: Option<&str>,
) -> Result<()> {
    client
        .execute_request(request, env_name)
        .await
        .context("Failed to execute WebSocket request")?;
    Ok(())
}

async fn execute_rsocket_request(
    client: &RSocketClient,
    request: &RSocketRequest,
    env_name: Option<&str>,
) -> Result<()> {
    client
        .execute_request(request, env_name)
        .await
        .context("Failed to execute RSocket request")?;
    Ok(())
}

async fn execute_graphql_request(
    client: &GraphQLClient,
    request: &GraphQLRequest,
    env_name: Option<&str>,
) -> Result<()> {
    println!("Query:\n{}", request.query);
    if let Some(vars) = &request.variables {
        println!("Variables:\n{}", serde_json::to_string_pretty(vars)?);
    }
    println!();

    let response = client
        .execute_request(request, env_name)
        .await
        .context("Failed to execute GraphQL request")?;

    client.print_response(&response);
    Ok(())
}
