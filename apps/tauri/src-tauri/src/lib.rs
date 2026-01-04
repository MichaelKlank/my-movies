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

fn start_embedded_server() {
    let runtime = get_runtime();

    runtime.spawn(async {
        tracing::info!("Starting embedded server...");

        let config = my_movies_server::ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            static_dir: None, // Tauri handles frontend via webview, no static serving needed
        };

        if let Err(e) = my_movies_server::start_server(config).await {
            tracing::error!("Server error: {}", e);
        }
    });

    tracing::info!("Embedded server started in background");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "info,my_movies_server=debug,my_movies_core=debug,my_movies_tauri=debug".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Start the embedded server before Tauri
    start_embedded_server();

    // Give the server a moment to start
    std::thread::sleep(std::time::Duration::from_millis(500));

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default();

    // Add barcode scanner plugin on mobile platforms
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        builder = builder.plugin(tauri_plugin_barcode_scanner::init());
    }

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
