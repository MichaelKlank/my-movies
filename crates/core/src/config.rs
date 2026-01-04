use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_database_url")]
    pub database_url: String,

    pub jwt_secret: String,

    pub tmdb_api_key: String,

    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_database_url() -> String {
    // Use compile-time CARGO_MANIFEST_DIR to find workspace root
    // This ensures both standalone server and Tauri app use the same database
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir);

    // Find workspace root (directory with Cargo.toml and crates/)
    if let Some(workspace_root) = path
        .ancestors()
        .find(|p| p.join("Cargo.toml").exists() && p.join("crates").exists())
    {
        let db_path = workspace_root.join("data").join("my-movies.db");
        return format!("sqlite:{}?mode=rwc", db_path.display());
    }

    // Fallback (should not happen in normal builds)
    "sqlite:./data/my-movies.db?mode=rwc".to_string()
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

impl Config {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env::<Config>()
    }

    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
