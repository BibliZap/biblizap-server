use actix_web::{post, App, HttpResponse, HttpServer, Responder};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RequestParameters {
    output_max_size: usize,
    depth: u8,
    input_id_list: Vec<String>
}

#[post("/api")]
async fn api(req_body: String) -> impl Responder {
    let parameters = serde_json::from_str::<RequestParameters>(&req_body).unwrap();
    let lens_api_key = "TdUUUOLUWn9HpA7zkZnu01NDYO1gVdVz71cDjFRQPeVDCrYGKWoY";

    let snowball = biblizap_rs::snowball(&parameters.input_id_list, parameters.depth, parameters.output_max_size, lens_api_key).await.unwrap();
    let f = format!("{:?}", snowball);
    HttpResponse::Ok().body(f)
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