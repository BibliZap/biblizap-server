use actix_web::{App, HttpServer, web};
use actix_web_static_files::ResourceFiles;
use biblizap_rs::lens::cache::postgres::PostgresBackend;
use config as conf;
use serde::Deserialize;
use std::env;
use thiserror::Error;

mod common;
mod denylist;
mod pubmed;
mod snowball;
mod tracking;

use pubmed::*;
use snowball::*;

// Includes the generated code for static files (frontend build).
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

/// Application configuration holding necessary secrets/settings.
struct AppConfig {
    lens_api_key: String,
    pubmed_api_key: Option<String>,
    cache_backend: PostgresBackend,
    database_pool: sqlx::PgPool,
}

/// Configuration that can be loaded from `biblizap.toml`.
#[derive(Debug, Deserialize, Default)]
struct FileConfig {
    lens_api_key: Option<String>,
    pubmed_api_key: Option<String>,
    cache_backend_url: Option<String>,
    bind_address: Option<String>,
    port: Option<u16>,
}

/// Custom error type for the server.
#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Biblizap(#[from] biblizap_rs::Error),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error(transparent)]
    DenyListError(#[from] denylist::DenyListError),
    #[error("Invalid identifier format: '{0}' is neither a valid DOI nor PMID")]
    InvalidIdFormat(String),
    #[error("Too many identifiers: maximum 7 allowed, got {0}")]
    TooManyIds(usize),
    #[error("No valid identifiers provided")]
    NoValidIds,
}

/// Main function to start the Actix-web server.
/// Parses command-line arguments for the API key and port,
/// loads the frontend static files, and serves the application.
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    dotenvy::dotenv().ok(); // Load .env file if present

    // Initialize logging: prefer RUST_LOG if set, otherwise use the CLI-provided log level.
    let mut logger_builder = env_logger::Builder::from_env(env_logger::Env::default());
    if std::env::var_os("RUST_LOG").is_none() {
        logger_builder.filter_level(args.log_level);
    }
    logger_builder.init();

    // Load configuration files (defaults < config file < env < CLI)
    // Precedence (highest -> lowest): ./biblizap.toml  >  $XDG_CONFIG_HOME/biblizap/biblizap.toml  >  /etc/biblizap/biblizap.toml
    // Use XDG_CONFIG_HOME if present, otherwise fall back to $HOME/.config
    let user_config_dir = env::var("XDG_CONFIG_HOME").ok().unwrap_or_else(|| {
        let home = env::var("HOME").unwrap_or_default();
        format!("{}/.config", home)
    });

    let builder = conf::Config::builder()
        .add_source(conf::File::with_name("/etc/biblizap/biblizap.toml").required(false))
        .add_source(
            conf::File::with_name(&format!("{}/biblizap/biblizap.toml", user_config_dir))
                .required(false),
        )
        .add_source(conf::File::with_name("biblizap.toml").required(false))
        .add_source(conf::Environment::with_prefix("BIBLIZAP").separator("__"));

    let settings = builder.build().unwrap_or_else(|e| {
        log::warn!("failed to build config: {}", e);
        conf::Config::default()
    });

    let file_cfg: FileConfig = settings.try_deserialize().unwrap_or_default();

    // Defaults
    const DEFAULT_BIND: &str = "127.0.0.1";
    const DEFAULT_PORT: u16 = 35642;

    // Merge: cli > config file > defaults
    let bind_address = args
        .bind_address
        .clone()
        .or(file_cfg.bind_address)
        .unwrap_or_else(|| DEFAULT_BIND.to_string());

    let port = args.port.or(file_cfg.port).unwrap_or(DEFAULT_PORT);

    // lens api key: CLI -> config file -> env var -> error
    let lens_api_key = args
        .lens_api_key
        .clone()
        .or(file_cfg.lens_api_key)
        .or_else(|| env::var("BIBLIZAP_LENS_API_KEY").ok())
        .unwrap_or_else(|| {
            log::error!(
                "Lens API key is required via CLI, config file, or BIBLIZAP_LENS_API_KEY env"
            );
            std::process::exit(1);
        });

    let cache_backend_url = args
        .cache_backend_url
        .clone()
        .or(file_cfg.cache_backend_url)
        .or_else(|| env::var("BIBLIZAP_CACHE_BACKEND_URL").ok())
        .unwrap_or_else(|| {
            log::error!(
                "Cache backend URL is required via CLI, config file, or BIBLIZAP_CACHE_BACKEND_URL env"
            );
            std::process::exit(1);
        });

    // For heavy IO workload with 4 workers per CPU
    let worker_count = num_cpus::get() * 4 * 2;

    // Create cache database connection pool with proper sizing for concurrent workers
    // Each worker may need to query the cache, so we allocate 2 connections per worker
    let cache_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(worker_count as u32)
        .connect(&cache_backend_url)
        .await
        .unwrap_or_else(|e| {
            log::error!("Unable to connect to the cache database: {}", e);
            std::process::exit(1);
        });

    log::info!(
        "Connected to cache database with {} max connections",
        worker_count
    );

    // Create PostgresBackend from pre-configured pool (runs migrations automatically)
    let cache_backend = biblizap_rs::lens::cache::postgres::PostgresBackend::from_pool(cache_pool)
        .await
        .unwrap_or_else(|e| {
            log::error!("Unable to initialize cache backend: {}", e);
            std::process::exit(1);
        });

    let tracking_database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        log::error!("Tracking database URL is required via DATABASE_URL env");
        std::process::exit(1);
    });

    // Create tracking database connection pool with proper sizing for concurrent workers
    let database_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(worker_count as u32)
        .connect(&tracking_database_url)
        .await
        .unwrap_or_else(|e| {
            log::error!("Unable to connect to tracking database: {}", e);
            std::process::exit(1);
        });

    log::info!("Connected to tracking database");

    // pubmed api key: CLI -> config file -> env var -> None (optional)
    let pubmed_api_key = args
        .pubmed_api_key
        .clone()
        .or(file_cfg.pubmed_api_key)
        .or_else(|| env::var("BIBLIZAP_PUBMED_API_KEY").ok());

    if pubmed_api_key.is_some() {
        log::info!("PubMed API key configured");
    } else {
        log::warn!(
            "No PubMed API key configured; PubMed searches will use unauthenticated access (rate-limited)"
        );
    }

    let config = web::Data::new(AppConfig {
        lens_api_key,
        pubmed_api_key,
        cache_backend,
        database_pool,
    });

    log::info!("Listening on http://{}:{}", bind_address, port);
    log::info!(
        "Running with {} workers (4x {} CPUs) for heavy IO workload",
        worker_count,
        num_cpus::get()
    );

    HttpServer::new(move || {
        let generated = generate();

        App::new()
            .app_data(config.clone())
            .service(web::resource("/api").route(web::post().to(snowball_request)))
            .service(web::resource("/api/pubmed_search").route(web::post().to(pubmed_search)))
            .service(
                web::resource("/api/denylist/download")
                    .route(web::get().to(denylist::download_denylist)),
            )
            .service(
                web::resource("/api/denylist/upload")
                    .route(web::post().to(denylist::upload_denylist)),
            )
            // Catch all route to serve frontend static files, with fallback to index.html for SPA routing
            .default_service(ResourceFiles::new("/", generated).resolve_not_found_to_root())
            .wrap(actix_web::middleware::Compress::default())
    })
    .workers(worker_count)
    .bind((bind_address, port))?
    .run()
    .await
}

use clap::Parser;

/// Run an instance of BibliZap
#[derive(Parser, Debug, Clone)]
#[command(
        version,
        about,
        long_about = None,
        after_long_help = color_print::cstr!(
r#"<bold><underline>Configuration:</underline></bold>
Configuration files are searched in the following order:
    ./biblizap.toml
    $XDG_CONFIG_HOME/biblizap/biblizap.toml (falls back to $HOME/.config/biblizap/biblizap.toml)
    /etc/biblizap/biblizap.toml

Environment variables with the prefix BIBLIZAP_ are also read (e.g. BIBLIZAP_LENS_API_KEY).

Values available in the config:
    - bind_address
    - port
    - lens_api_key
    - cache_backend_url

Secrets (Lens API key and Cache URL): prefer keeping `biblizap.toml` file mode 600, or set BIBLIZAP_LENS_API_KEY.

CLI flags override config and env."#),
)]
struct Args {
    /// Your Lens.org API key (optional; can come from config or env)
    #[arg(short, long)]
    lens_api_key: Option<String>,

    /// Your PubMed E-Utilities API key (optional; can come from config or env)
    #[arg(long)]
    pubmed_api_key: Option<String>,

    /// An URL to a working postgresql cache database (optional; can come from config or env)
    #[arg(short, long)]
    cache_backend_url: Option<String>,

    /// Address to bind the server (optional; overrides config)
    #[arg(short, long)]
    bind_address: Option<String>,

    /// Port on which to listen (optional; overrides config)
    #[arg(short, long)]
    port: Option<u16>,

    /// Log level for the application
    #[arg(short = 'L', long, default_value_t = log::LevelFilter::Info)]
    log_level: log::LevelFilter,
}
