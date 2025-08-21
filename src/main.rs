use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use biblizap_rs::SearchFor;
use serde::Deserialize;
use thiserror::Error;

// Includes the generated code for static files (frontend build).
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

/// Application configuration holding necessary secrets/settings.
struct AppConfig {
    lens_api_key: String,
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
    #[error("snowball error")]
    Biblizap(#[from] biblizap_rs::Error),
    #[error("invalid parameters")]
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
    log::debug!("Sending {} articles, {} characters response", snowball.len(), json_str.len());

    Ok(json_str)
}

/// Actix-web handler for the `/api` endpoint.
/// Receives the request body, extracts parameters, performs the snowball search,
/// and returns the results as JSON or an error response.
async fn api(req_body: String, _: HttpRequest, config: web::Data<AppConfig>) -> impl Responder {
    let snowball = handle_request(&req_body, &config.lens_api_key).await;

    match snowball {
        Ok(snowball) => HttpResponse::Ok().body(snowball),
        Err(error) => HttpResponse::InternalServerError().body(format!("{error:#?}")),
    }
}

/// Main function to start the Actix-web server.
/// Parses command-line arguments for the API key and port,
/// loads the frontend static files, and serves the application.
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let config = web::Data::new(AppConfig {
        lens_api_key: args.lens_api_key,
    });

    // Initialize logging: prefer RUST_LOG if set, otherwise use the CLI-provided log level.
    let mut logger_builder = env_logger::Builder::from_env(env_logger::Env::default());
    if std::env::var_os("RUST_LOG").is_none() {
        logger_builder.filter_level(args.log_level);
    }
    logger_builder.init();

    log::info!("Listening on http://127.0.0.1:{}", args.port);

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
    .bind(("127.0.0.1", args.port))?
    .run()
    .await
}

use clap::Parser;

/// Run an instance of BibliZap
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Your Lens.org API key
    #[arg(short, long)]
    lens_api_key: String,

    /// Port on which to listen
    #[arg(short, long, default_value_t = 35642)]
    port: u16,

    /// Log level for the application
    #[arg(short, long, default_value_t = log::LevelFilter::Info)]
    log_level: log::LevelFilter,
}
