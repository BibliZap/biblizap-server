use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use biblizap_rs::SearchFor;
use config as conf;
use serde::Deserialize;
use std::env;
use thiserror::Error;

// Includes the generated code for static files (frontend build).
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

/// Application configuration holding necessary secrets/settings.
struct AppConfig {
    lens_api_key: String,
}

/// Configuration that can be loaded from `biblizap.toml`.
#[derive(Debug, Deserialize, Default)]
struct FileConfig {
    lens_api_key: Option<String>,
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
}

/// Handles the core logic of performing the snowball search using biblizap-rs.
/// Takes the request body (JSON string) and the Lens API key.
/// Returns a JSON string representing the search results or an error.
async fn handle_request(req_body: &str, lens_api_key: &str) -> Result<String, Error> {
    let parameters = serde_json::from_str::<SnowballParameters>(req_body)?;
    log::info!("Received request: {:?}", parameters);
    let snowball = biblizap_rs::snowball(
        &parameters.input_id_list,
        parameters.depth.clamp(1, 3),
        parameters
            .output_max_size
            .parse::<usize>()
            .unwrap_or(usize::MAX)
            .clamp(1, usize::MAX),
        &parameters.search_for,
        lens_api_key,
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
async fn api(req_body: String, _: HttpRequest, config: web::Data<AppConfig>) -> impl Responder {
    let snowball: Result<String, Error> = handle_request(&req_body, &config.lens_api_key).await;

    match snowball {
        Ok(snowball) => {
            log::info!("Request completed successfully");
            HttpResponse::Ok().body(snowball)
        }
        Err(error) => {
            log::error!("Request failed: {error:?}");
            HttpResponse::InternalServerError().body(format!("{error}"))
        }
    }
}

/// Main function to start the Actix-web server.
/// Parses command-line arguments for the API key and port,
/// loads the frontend static files, and serves the application.
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

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

    let config = web::Data::new(AppConfig { lens_api_key });

    log::info!("Listening on http://{}:{}", bind_address, port);

    HttpServer::new(move || {
        let generated = generate();

        App::new()
            .service(
                web::resource("/api")
                    .app_data(config.clone())
                    .route(web::post().to(api)),
            )
            .service(actix_web_static_files::ResourceFiles::new("/", generated))
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

Secrets (Lens API key): prefer keeping `biblizap.toml` file mode 600, or set BIBLIZAP_LENS_API_KEY.

CLI flags override config and env."#),
)]
struct Args {
    /// Your Lens.org API key (optional; can come from config or env)
    #[arg(short, long)]
    lens_api_key: Option<String>,

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
