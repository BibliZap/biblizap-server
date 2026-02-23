use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, web};
use actix_web_static_files::ResourceFiles;
use biblizap_rs::{SearchFor, lens::cache::postgres::PostgresBackend};
use config as conf;
use serde::Deserialize;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

mod tracking;

// Includes the generated code for static files (frontend build).
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

/// Application configuration holding necessary secrets/settings.
struct AppConfig {
    lens_api_key: String,
    cache_backend: PostgresBackend,
    tracking_pool: sqlx::PgPool,
}

/// Configuration that can be loaded from `biblizap.toml`.
#[derive(Debug, Deserialize, Default)]
struct FileConfig {
    lens_api_key: Option<String>,
    cache_backend_url: Option<String>,
    bind_address: Option<String>,
    port: Option<u16>,
}

/// Parameters received from the frontend for the snowball search.
#[derive(Debug, Deserialize)]
struct SnowballParameters {
    output_max_size: String,
    depth: u8,
    input_id_list: Vec<String>,
    search_for: SearchFor,
}

/// Custom error type for the server.
#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Biblizap(#[from] biblizap_rs::Error),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error("Invalid identifier format: '{0}' is neither a valid DOI nor PMID")]
    InvalidIdFormat(String),
    #[error("Too many identifiers: maximum 7 allowed, got {0}")]
    TooManyIds(usize),
    #[error("No valid identifiers provided")]
    NoValidIds,
}

/// Validates if a string is a valid DOI.
/// DOIs start with "10." followed by at least 4 digits, a "/", and a suffix.
fn is_valid_doi(s: &str) -> bool {
    s.starts_with("10.") 
        && s.len() > 7  // Minimum: "10.1234/x"
        && s.contains('/') 
        && s.chars().skip(3).take_while(|c| c.is_ascii_digit()).count() >= 4
}

/// Validates if a string is a valid PMID.
/// PMIDs are purely numeric identifiers.
fn is_valid_pmid(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// Validates if a string is either a valid DOI or PMID.
fn is_valid_id(s: &str) -> bool {
    is_valid_doi(s) || is_valid_pmid(s)
}

fn epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Handles the core logic of performing the snowball search using biblizap-rs.
/// Takes the request body (JSON string) and the Lens API key.
/// Returns a JSON string representing the search results or an error.
async fn handle_request(
    req_body: &str,
    lens_api_key: &str,
    cache_backend: &PostgresBackend,
) -> Result<String, Error> {
    let parameters = serde_json::from_str::<SnowballParameters>(req_body)?;
    log::info!("Received request: {:?}", parameters);
    
    // Server-side validation: check max 7 IDs
    if parameters.input_id_list.len() > 7 {
        return Err(Error::TooManyIds(parameters.input_id_list.len()));
    }
    
    // Server-side validation: ensure at least one ID
    if parameters.input_id_list.is_empty() {
        return Err(Error::NoValidIds);
    }
    
    // Server-side validation: check each ID is valid DOI or PMID
    for id in &parameters.input_id_list {
        if !is_valid_id(id) {
            return Err(Error::InvalidIdFormat(id.clone()));
        }
    }
    let snowball = biblizap_rs::snowball(
        &parameters.input_id_list,
        parameters.depth.clamp(1, 2),
        parameters
            .output_max_size
            .parse::<usize>()
            .unwrap_or(usize::MAX)
            .clamp(1, usize::MAX),
        &parameters.search_for,
        lens_api_key,
        None,
        Some(cache_backend),
    )
    .await?;

    let json_str = serde_json::to_string(&snowball)?;
    log::debug!(
        "Sending {} articles, {} characters response",
        snowball.len(),
        json_str.len()
    );

    Ok(json_str)
}

/// Actix-web handler for the `/api` endpoint.
/// Receives the request body, extracts parameters, performs the snowball search,
/// and returns the results as JSON or an error response.
async fn api(req_body: String, req: HttpRequest, config: web::Data<AppConfig>) -> impl Responder {
    let request_started_ms = epoch_ms();
    let request_inputs = serde_json::from_str::<serde_json::Value>(&req_body).ok();
    let snowball: Result<String, Error> =
        handle_request(&req_body, &config.lens_api_key, &config.cache_backend).await;
    let request_completed_ms = epoch_ms();
    let request_duration_ms = request_completed_ms.saturating_sub(request_started_ms) as i32;

    // Extract bbz_sid from cookie for event logging
    let bbz_sid = req
        .cookie("bbz_sid")
        .and_then(|cookie| Uuid::parse_str(cookie.value()).ok());

    match snowball {
        Ok(snowball) => {
            log::info!("Request completed successfully");
            
            // Log event asynchronously (don't block response)
            if let Some(sid) = bbz_sid {
                let pool = config.tracking_pool.clone();
                let article_count = snowball.matches("\"doi\":").count();
                tracking::log_search_success(
                    sid,
                    article_count,
                    request_started_ms,
                    request_completed_ms,
                    request_duration_ms,
                    request_inputs.clone(),
                    pool,
                );
            }
            
            HttpResponse::Ok().body(snowball)
        }
        Err(error) => {
            log::error!("Request failed: {error:?}");
            
            // Log error event asynchronously
            if let Some(sid) = bbz_sid {
                let pool = config.tracking_pool.clone();
                let error_msg = error.to_string();
                tracking::log_search_error(
                    sid,
                    error_msg,
                    request_started_ms,
                    request_completed_ms,
                    request_duration_ms,
                    request_inputs.clone(),
                    pool,
                );
            }
            
            // Return 400 Bad Request for validation errors, 500 for others
            match error {
                Error::InvalidIdFormat(_) | Error::TooManyIds(_) | Error::NoValidIds => {
                    HttpResponse::BadRequest().body(format!("{error}"))
                }
                _ => HttpResponse::InternalServerError().body(format!("{error}")),
            }
        }
    }
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

    let cache_backend =
        biblizap_rs::lens::cache::postgres::PostgresBackend::from_url(&cache_backend_url)
            .await
            .unwrap_or_else(|_| {
                log::error!("Unable to connect to the cache database");
                std::process::exit(1);
            });

    let tracking_database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            log::error!(
                "Tracking database URL is required via DATABASE_URL env"
            );
            std::process::exit(1);
        });

    // Create tracking database connection pool
    let tracking_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&tracking_database_url)
        .await
        .unwrap_or_else(|e| {
            log::error!("Unable to connect to tracking database: {}", e);
            std::process::exit(1);
        });

    log::info!("Connected to tracking database");

    let config = web::Data::new(AppConfig {
        lens_api_key,
        cache_backend,
        tracking_pool,
    });

    log::info!("Listening on http://{}:{}", bind_address, port);

    HttpServer::new(move || {
        let generated = generate();

        App::new()
            .app_data(config.clone())
            .service(
                web::resource("/api")
                    .route(web::post().to(api)),
            )
            .service(
                web::resource("/link")
                    .route(web::post().to(tracking::link_handler)),
            )
            .service(ResourceFiles::new("/", generated))
    })
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
    #[arg(short, long, default_value_t = log::LevelFilter::Info)]
    log_level: log::LevelFilter,
}
