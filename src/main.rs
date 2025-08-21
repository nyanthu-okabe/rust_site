use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use serde::Deserialize;
use reqwest;
use lol_html::{element, rewrite_str, Settings};
use url::Url;

#[derive(Deserialize)]
struct QueryParams {
    url: String,
}

async fn web_html_viewer(query: web::Query<QueryParams>) -> impl Responder {
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
                    Err(_) => return HttpResponse::InternalServerError().body("Failed to read response text"),
                };

                let element_content_handlers = vec![
                    element!("*[href]", |el| {
                        if let Some(href) = el.get_attribute("href") {
                            if !href.starts_with("/web_html_viewer?url=") {
                                if let Ok(absolute_url) = base_url.join(&href) {
                                    let proxied_url = format!("/web_html_viewer?url={}", absolute_url);
                                    el.set_attribute("href", &proxied_url).unwrap();
                                }
                            }
                        }
                        Ok(())
                    }),
                    element!("*[src]", |el| {
                        if let Some(src) = el.get_attribute("src") {
                            if !src.starts_with("/web_html_viewer?url=") {
                                if let Ok(absolute_url) = base_url.join(&src) {
                                    let proxied_url = format!("/web_html_viewer?url={}", absolute_url);
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
                    Err(_) => HttpResponse::InternalServerError().body("Failed to read response bytes"),
                }
            }
        }
        Err(_) => HttpResponse::InternalServerError().body("Failed to fetch URL"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/web_html_viewer", web::get().to(web_html_viewer))
    })
    .bind("127.0.0.1:3142")?
    .run()
    .await
}
