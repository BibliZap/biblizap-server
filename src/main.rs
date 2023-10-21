use actix_web::{post, App, HttpResponse, HttpServer, Responder};
use biblizap_rs::SearchFor;
use serde::Deserialize;
use thiserror::Error;

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

async fn handle_request(req_body: &str) -> Result<String, Error> {
    let parameters = serde_json::from_str::<SnowballParameters>(req_body)?;

    let lens_api_key = "TdUUUOLUWn9HpA7zkZnu01NDYO1gVdVz71cDjFRQPeVDCrYGKWoY";

    let snowball = biblizap_rs::snowball(&parameters.input_id_list, parameters.depth.clamp(1, 3), parameters.output_max_size.clamp(1, 3000), &parameters.search_for, lens_api_key).await?;

    let json_str = serde_json::to_string(&snowball)?;

    Ok(json_str)
}

#[post("/api")]
async fn api(req_body: String) -> impl Responder {
    let snowball = handle_request(&req_body).await;
    
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
    HttpServer::new(|| {
        App::new()
            .service(api)
            .service(actix_files::Files::new("/","./biblizap-frontend/dist").index_file("index.html"))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}