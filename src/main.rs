use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use biblizap_rs::SearchFor;
use serde::Deserialize;
use thiserror::Error;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

struct AppConfig {
    lens_api_key: String
}

#[derive(Debug, Deserialize)]
struct SnowballParameters {
    output_max_size: usize,
    depth: u8,
    input_id_list: Vec<String>,
    search_for: SearchFor
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("snowball error")]
    Biblizap(#[from] biblizap_rs::Error),
    #[error("invalid parameters")]
    JsonError(#[from] serde_json::Error),
}

async fn handle_request(req_body: &str, lens_api_key: &str) -> Result<String, Error> {
    let parameters = serde_json::from_str::<SnowballParameters>(req_body)?;

    let snowball = biblizap_rs::snowball(&parameters.input_id_list, parameters.depth.clamp(1, 3), parameters.output_max_size.clamp(1, 3000), &parameters.search_for, lens_api_key).await?;

    let json_str = serde_json::to_string(&snowball)?;

    Ok(json_str)
}

async fn api(req_body: String, _: HttpRequest, config: web::Data<AppConfig>) -> impl Responder {
    let snowball = handle_request(&req_body, &config.lens_api_key).await;
    
    match snowball {
        Ok(snowball) => {
            HttpResponse::Ok().body(snowball)
        }
        Err(error) => {
            HttpResponse::InternalServerError().body(format!("{:#?}", error))
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let config = web::Data::new(AppConfig {lens_api_key: args.lens_api_key}); 

    HttpServer::new(move || {
        let generated = generate();

        App::new()
            .service(web::resource("/api")
                .app_data(config.clone())
                .route(web::post().to(api)))
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
    #[arg(short, long, default_value_t = 8080)]
    port: u16
}