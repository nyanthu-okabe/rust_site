use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use lol_html::{Settings, element, rewrite_str};
use rand::Rng;
use reqwest;
use serde::Deserialize;
use url::Url;

#[derive(Deserialize)]
struct QueryParams {
    url: String,
}

async fn whv(query: web::Query<QueryParams>) -> impl Responder {
    let mut request_url = query.url.clone();
    if !request_url.starts_with("http://") && !request_url.starts_with("https://") {
        request_url = format!("https://{}", request_url);
    }

    let base_url = match Url::parse(&request_url) {
        Ok(url) => url,
        Err(_) => return HttpResponse::BadRequest().body("Invalid base URL"),
    };

    match reqwest::get(&request_url).await {
        Ok(response) => {
            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();

            if content_type.starts_with("text/html") {
                let body = match response.text().await {
                    Ok(text) => text,
                    Err(_) => {
                        return HttpResponse::InternalServerError()
                            .body("Failed to read response text");
                    }
                };

                let element_content_handlers = vec![
                    element!("*[href]", |el| {
                        if let Some(href) = el.get_attribute("href") {
                            if !href.starts_with("/whv?url=") {
                                if let Ok(absolute_url) = base_url.join(&href) {
                                    let proxied_url = format!("/whv?url={}", absolute_url);
                                    el.set_attribute("href", &proxied_url).unwrap();
                                }
                            }
                        }
                        Ok(())
                    }),
                    element!("*[src]", |el| {
                        if let Some(src) = el.get_attribute("src") {
                            if !src.starts_with("/whv?url=") {
                                if let Ok(absolute_url) = base_url.join(&src) {
                                    let proxied_url = format!("/whv?url={}", absolute_url);
                                    el.set_attribute("src", &proxied_url).unwrap();
                                }
                            }
                        }
                        Ok(())
                    }),
                ];

                let settings = Settings {
                    element_content_handlers,
                    ..Settings::default()
                };

                let output = rewrite_str(&body, settings).unwrap();
                HttpResponse::Ok().content_type("text/html").body(output)
            } else {
                match response.bytes().await {
                    Ok(bytes) => HttpResponse::Ok().content_type(content_type).body(bytes),
                    Err(_) => {
                        HttpResponse::InternalServerError().body("Failed to read response bytes")
                    }
                }
            }
        }
        Err(_) => HttpResponse::InternalServerError().body("Failed to fetch URL"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 100000から999999の間のランダムな6桁のポート番号を生成
    let port = rand::thread_rng().gen_range(1000..=9999);
    let bind_address = format!("0.0.0.0:{}", port);

    println!("Server is running at: {}", bind_address);

    HttpServer::new(|| App::new().route("/whv", web::get().to(whv)))
        .bind(bind_address)?
        .run()
        .await
}
