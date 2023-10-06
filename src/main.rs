use actix_web::{post, App, HttpResponse, HttpServer, Responder};
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Deserialize)]
struct RequestParameters {
    output_max_size: usize,
    depth: u8,
    input_id_list: Vec<String>
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("snowball error")]
    Biblizap(#[from] biblizap_rs::Error),
    #[error("invalid parameters")]
    JsonError(#[from] serde_json::Error),
}

async fn handle_request(req_body: &str) -> Result<String, Error> {
    let parameters = serde_json::from_str::<RequestParameters>(req_body)?;

    let lens_api_key = "TdUUUOLUWn9HpA7zkZnu01NDYO1gVdVz71cDjFRQPeVDCrYGKWoY";

    let snowball = biblizap_rs::snowball(&parameters.input_id_list, parameters.depth, parameters.output_max_size, lens_api_key).await?;

    let json_str = serde_json::to_string(&snowball)?;

    Ok(json_str)
}

#[post("/api")]
async fn api(req_body: String) -> impl Responder {
    let snowball = handle_request(&req_body).await;
    
    match snowball {
        Ok(snowball) => {
            let f = serde_json::to_string(&snowball).unwrap();
            HttpResponse::Ok().body(f)
        }
        Err(error) => {
            HttpResponse::InternalServerError().body(error.to_string())
        }
    }
    
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(api)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}