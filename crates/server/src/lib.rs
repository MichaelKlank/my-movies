use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post},
};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use my_movies_core::{
    Config,
    db::create_pool,
    services::{
        AuthService, CollectionService, EanService, ImportService, MovieService, SeriesService,
        SettingsService, TmdbService,
    },
};

pub mod middleware;
pub mod routes;

use routes::{auth, collections, import, movies, scan, series, settings, users, ws};

pub struct AppState {
    pub auth_service: AuthService,
    pub movie_service: MovieService,
    pub series_service: SeriesService,
    pub collection_service: CollectionService,
    pub tmdb_service: TmdbService,
    pub ean_service: EanService,
    pub import_service: ImportService,
    pub settings_service: SettingsService,
    pub ws_broadcast: tokio::sync::broadcast::Sender<String>,
}

/// Configuration for starting the server
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    /// Directory containing the frontend static files (index.html, assets, etc.)
    /// If None, no static files are served (API-only mode)
    pub static_dir: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            static_dir: None,
        }
    }
}

/// Creates the application state with all services initialized
pub async fn create_app_state(config: &Config) -> anyhow::Result<Arc<AppState>> {
    // Create database pool
    let pool = create_pool(&config.database_url).await?;
    tracing::info!("Database connected");

    // Create broadcast channel for WebSocket
    let (ws_tx, _) = tokio::sync::broadcast::channel::<String>(100);

    // Create settings service first to get TMDB API key
    let settings_service = SettingsService::new(pool.clone());

    // Get TMDB API key from settings (env var has priority, then database)
    let tmdb_api_key = settings_service
        .get(my_movies_core::models::SettingKey::TmdbApiKey)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| config.tmdb_api_key.clone());

    // Create services
    let state = Arc::new(AppState {
        auth_service: AuthService::new(pool.clone(), config.jwt_secret.clone()),
        movie_service: MovieService::new(pool.clone()),
        series_service: SeriesService::new(pool.clone()),
        collection_service: CollectionService::new(pool.clone()),
        tmdb_service: TmdbService::new(tmdb_api_key),
        ean_service: EanService::new(),
        import_service: ImportService::new(pool.clone()),
        settings_service,
        ws_broadcast: ws_tx,
    });

    Ok(state)
}

/// Creates the router with all routes configured
pub fn create_router(state: Arc<AppState>, static_dir: Option<&str>) -> Router {
    let mut router = Router::new()
        // Public routes
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/forgot-password", post(auth::forgot_password))
        .route("/api/v1/auth/reset-password", post(auth::reset_password))
        .route("/health", get(health_check))
        // Protected routes
        .nest("/api/v1", protected_routes(state.clone()))
        // WebSocket
        .route("/ws", get(ws::websocket_handler))
        // Serve uploaded files (posters, etc.)
        .nest_service("/uploads", ServeDir::new("uploads"))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Serve static frontend files if directory is configured
    if let Some(dir) = static_dir {
        let index_path = format!("{}/index.html", dir);
        if std::path::Path::new(&index_path).exists() {
            tracing::info!("Serving static files from: {}", dir);
            // Serve static files, with fallback to index.html for SPA routing
            router = router.fallback_service(
                ServeDir::new(dir).not_found_service(ServeFile::new(&index_path)),
            );
        } else {
            tracing::warn!(
                "Static directory configured but index.html not found: {}",
                dir
            );
        }
    }

    router
}

fn protected_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        // Auth
        .route("/auth/me", get(auth::me))
        // Movies
        .route("/movies", get(movies::list).post(movies::create))
        .route("/movies/check-duplicates", get(movies::check_duplicates))
        .route("/movies/duplicates", get(movies::find_all_duplicates))
        .route(
            "/movies/:id",
            get(movies::get).put(movies::update).delete(movies::delete),
        )
        .route("/movies/:id/refresh-tmdb", post(movies::refresh_tmdb))
        .route("/movies/:id/upload-poster", post(movies::upload_poster))
        // Series
        .route("/series", get(series::list).post(series::create))
        .route(
            "/series/:id",
            get(series::get).put(series::update).delete(series::delete),
        )
        // Collections
        .route(
            "/collections",
            get(collections::list).post(collections::create),
        )
        .route(
            "/collections/:id",
            get(collections::get)
                .put(collections::update)
                .delete(collections::delete),
        )
        .route(
            "/collections/:id/items",
            get(collections::get_items).post(collections::add_item),
        )
        .route(
            "/collections/:id/items/:item_id",
            delete(collections::remove_item),
        )
        // Scanning & Lookup
        .route("/scan", post(scan::lookup_barcode))
        .route("/tmdb/search/movies", get(scan::search_tmdb_movies))
        .route("/tmdb/search/tv", get(scan::search_tmdb_tv))
        .route("/tmdb/movies/:id", get(scan::get_tmdb_movie))
        .route("/tmdb/tv/:id", get(scan::get_tmdb_tv))
        // Import/Export
        .route("/import/csv", post(import::import_csv))
        .route("/import/enrich-tmdb", post(import::enrich_movies_tmdb))
        // Settings (admin only)
        .route("/settings", get(settings::get_settings))
        .route(
            "/settings/:key",
            axum::routing::put(settings::update_setting),
        )
        .route("/settings/test/tmdb", post(settings::test_tmdb))
        // User management (admin only)
        .route("/users", get(users::list_users))
        .route(
            "/users/:id/role",
            axum::routing::put(users::update_user_role),
        )
        .route("/users/:id", delete(users::delete_user))
        .route(
            "/users/:id/password",
            axum::routing::put(users::admin_set_password),
        )
        .layer(axum::middleware::from_fn_with_state(
            state,
            middleware::auth::auth_middleware,
        ))
}

async fn health_check() -> &'static str {
    "OK"
}

/// Starts the server and blocks until shutdown
///
/// This is the main entry point for running the server standalone
pub async fn start_server(server_config: ServerConfig) -> anyhow::Result<()> {
    // Load .env file
    dotenvy::dotenv().ok();

    // Load app config
    let config = Config::from_env().map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

    tracing::info!("Starting My Movies server...");

    // Create app state
    let state = create_app_state(&config).await?;

    // Build router with optional static file serving
    let app = create_router(state, server_config.static_dir.as_deref());

    // Start server
    let addr = format!("{}:{}", server_config.host, server_config.port);
    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Starts the server in the background (for embedding in Tauri)
///
/// Returns a handle that can be used to check if the server is still running
pub fn start_server_background(
    server_config: ServerConfig,
) -> tokio::task::JoinHandle<anyhow::Result<()>> {
    tokio::spawn(async move { start_server(server_config).await })
}
