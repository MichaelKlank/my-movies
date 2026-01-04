use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use my_movies_server::ServerConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file early for environment variables
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,my_movies_server=debug,my_movies_core=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Configure server
    let server_config = ServerConfig {
        host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
        port: std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000),
        // STATIC_DIR environment variable to serve frontend files
        static_dir: std::env::var("STATIC_DIR").ok(),
    };

    my_movies_server::start_server(server_config).await
}
