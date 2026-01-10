use std::sync::OnceLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Global tokio runtime for the embedded server
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn get_runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    })
}

fn setup_app_environment() {
    // Get the app's data directory (~/Library/Application Support/com.mymovies.desktop on macOS)
    let data_dir = dirs::data_dir()
        .map(|d| d.join("com.mymovies.desktop"))
        .unwrap_or_else(|| std::path::PathBuf::from("./data"));

    // Create data directory if it doesn't exist
    std::fs::create_dir_all(&data_dir).ok();

    // SAFETY: std::env::set_var() is marked unsafe because it can cause data races
    // when called from multiple threads. However, we call this function at app startup
    // in the main thread before any other threads are spawned, so it's safe here.
    unsafe {
        // Set DATABASE_URL if not already set
        if std::env::var("DATABASE_URL").is_err() {
            let db_path = data_dir.join("my-movies.db");
            let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
            std::env::set_var("DATABASE_URL", &db_url);
            tracing::info!("Database path: {}", db_path.display());
        }

        // Set default JWT_SECRET if not set (for desktop app, a static secret is acceptable)
        if std::env::var("JWT_SECRET").is_err() {
            // This is a fallback for desktop use - in production server deployments, use a proper secret
            std::env::set_var(
                "JWT_SECRET",
                "my-movies-desktop-jwt-secret-change-in-production",
            );
        }

        // Set default TMDB_API_KEY if not set
        // Users should set this in their environment for full functionality
        if std::env::var("TMDB_API_KEY").is_err() {
            // Empty key - TMDB features won't work until user configures it
            std::env::set_var("TMDB_API_KEY", "");
            tracing::warn!("TMDB_API_KEY not set - movie metadata lookup will be disabled");
        }
    }
}

fn start_embedded_server() {
    let runtime = get_runtime();

    runtime.spawn(async {
        tracing::info!("Starting embedded server...");

        let config = my_movies_server::ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            static_dir: None, // Tauri WebView serves frontend, server is API-only
        };

        if let Err(e) = my_movies_server::start_server(config).await {
            tracing::error!("Server error: {}", e);
        }
    });

    tracing::info!("Embedded server started in background");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing (disable ANSI colors for Xcode compatibility)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "info,my_movies_server=debug,my_movies_core=debug,my_movies_tauri=debug".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().with_ansi(false))
        .init();

    // Setup environment variables for the app
    setup_app_environment();

    // Start the embedded server before Tauri
    start_embedded_server();

    // Give the server a moment to start
    std::thread::sleep(std::time::Duration::from_millis(500));

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_http::init());

    // Add barcode scanner plugin on mobile platforms
    // Note: The {} block is required because #[cfg] cannot be applied directly to assignments
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        builder = builder.plugin(tauri_plugin_barcode_scanner::init());
    }

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
