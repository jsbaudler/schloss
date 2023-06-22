use actix_web::cookie::time::Duration;
use actix_web::{get, post, web, App, HttpResponse, HttpServer};
use serde::Deserialize;
use serde_json::json;
use std::env;

#[derive(Deserialize)]
struct AuthRequest {
    shared_secret: String,
    service_url: String,
}

#[get("/")]
async fn index() -> HttpResponse {
    let rendered = r#"
        <!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <title>Schloss</title>
        </head>
        <body>
            <h1>This is Schloss 🔒</h1>
        </body>
        </html>
    "#;

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(rendered)
}

// generate the auth cookie for the domain schloss is deployed on
#[post("/generate_auth_cookie")]
async fn generate_auth_cookie(form: web::Form<AuthRequest>) -> HttpResponse {
    let expected_shared_secret = env::var("SHARED_SECRET").unwrap_or("shared_secret".to_string());
    let provided_shared_secret = form.shared_secret.clone();
    let service_url = form.service_url.clone();

    if provided_shared_secret == expected_shared_secret {
        let domain = env::var("DOMAIN").unwrap_or_else(|_| "127.0.0.1".to_string());
        let token_name = env::var("TOKEN_NAME").unwrap_or("test_auth_token".to_string());
        let token_value = env::var("TOKEN_VALUE").unwrap_or("abcdefgh1234".to_string());

        let auth_cookie = actix_web::cookie::CookieBuilder::new(&token_name, &token_value)
            .domain(domain)
            .path("/")
            .max_age(Duration::new(24 * 60 * 60, 0))
            .secure(true)
            .http_only(true)
            .finish();

        let redirect_url = format!("{}", service_url);

        let response = HttpResponse::Found()
            .cookie(auth_cookie)
            .append_header(("location", redirect_url))
            .finish();

        return response;
    }

    HttpResponse::Unauthorized().body("Unauthorized")
}

// register the current lock with the schluessel entrypoint
async fn register_instance() -> Result<(), reqwest::Error> {
    let schluessel_endpoint = env::var("SCHLUESSEL_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:8080/register".to_string());

    let domain = env::var("DOMAIN").unwrap_or_else(|_| "127.0.0.1".to_string());
    let services = env::var("SERVICES").unwrap_or_else(|_| {
        json!([
            ["Service1", "http://127.0.0.1:8081"],
            ["Service2", "http://127.0.0.1:8082"]
        ])
        .to_string()
    });

    let data = json!({
        "domain": domain,
        "services": serde_json::from_str::<Vec<Vec<String>>>(&services).unwrap_or_default(),
    });

    let client = reqwest::Client::new();
    client.post(&schluessel_endpoint).json(&data).send().await?;

    Ok(())
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let http_host = env::var("HTTP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let http_port = env::var("HTTP_PORT").unwrap_or_else(|_| "8080".to_string());

    let bind_address = format!("{}:{}", http_host, http_port);

    if let Err(e) = register_instance().await {
        eprintln!("Failed to register instance: {}", e);
    }

    HttpServer::new(|| App::new().service(index).service(generate_auth_cookie))
        .bind(bind_address)?
        .run()
        .await
}
