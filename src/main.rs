use actix_web::cookie::time::Duration;
use actix_web::{get, middleware::Logger, post, web, App, HttpRequest, HttpResponse, HttpServer};
use env_logger::Env;
use serde::Deserialize;
use serde_json::json;
use std::{env, process};

/// Request body for the "/generate_auth_cookie" route
#[derive(Deserialize)]
struct AuthRequest {
    shared_secret: String,
    service_url: String,
}

/// Route handler for "/"
/// If the request includes a valid auth cookie, it will respond with a list of services.
/// Otherwise, it will respond with "Unauthorized".
#[get("/")]
async fn index(req: HttpRequest) -> HttpResponse {
    let token_name = env::var("TOKEN_NAME").unwrap_or("test_auth_token".to_string());
    let token_value = env::var("TOKEN_VALUE").unwrap_or("abcdefgh1234".to_string());

    let auth_cookie = req.cookie(&token_name);

    if let Some(cookie) = auth_cookie {
        if cookie.value() == token_value {
            let services = env::var("SERVICES").unwrap_or_else(|_| {
                json!([
                    ["Service1", "http://127.0.0.1:8081"],
                    ["Service2", "http://127.0.0.1:8082"]
                ])
                .to_string()
            });

            let rendered = format!(
                r#"
                <!DOCTYPE html>
                <html lang="en">
                <head>
                    <meta charset="UTF-8">
                    <title>🔒 Schloss 🔒</title>
                </head>
                <body>
                    <h1>This is Schloss 🔒</h1>
                    <h2>Services:</h2>
                    <ul>
                        {}
                    </ul>
                </body>
                </html>
            "#,
                render_services(&services)
            );

            return HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(rendered);
        }
    }

    HttpResponse::Unauthorized().body("Unauthorized")
}

/// Generates HTML list of services
fn render_services(services: &str) -> String {
    let services_list: Vec<Vec<String>> = serde_json::from_str(services).unwrap_or_default();
    let mut rendered_services = String::new();

    for service in services_list {
        let service_name = &service[0];
        let service_url = &service[1];
        rendered_services.push_str(&format!(
            "<li><a href='http://{}'>{}</a></li>",
            service_url, service_name
        ));
    }

    rendered_services
}

/// Route handler for "/generate_auth_cookie"
/// If the request includes the correct shared secret, it will generate an auth cookie and redirect to the service URL.
/// Otherwise, it will respond with "Unauthorized".
#[post("/generate_auth_cookie")]
async fn generate_auth_cookie(form: web::Form<AuthRequest>) -> HttpResponse {
    let expected_shared_secret = env::var("SHARED_SECRET").unwrap_or("shared_secret".to_string());
    let provided_shared_secret = form.shared_secret.clone();
    let service_url = form.service_url.clone();

    if provided_shared_secret == expected_shared_secret {
        log::info!("Authorized request detected.");

        let domain = env::var("DOMAIN").unwrap_or_else(|_| "127.0.0.1".to_string());
        let token_name = env::var("TOKEN_NAME").unwrap_or("test_auth_token".to_string());
        let token_value = env::var("TOKEN_VALUE").unwrap_or("abcdefgh1234".to_string());

        log::info!("Creating auth cookie for domain {domain}");

        let auth_cookie = actix_web::cookie::CookieBuilder::new(&token_name, &token_value)
            .domain(domain)
            .path("/")
            .max_age(Duration::new(24 * 60 * 60, 0))
            .secure(true)
            .http_only(true)
            .finish();

        let response = HttpResponse::Found()
            .cookie(auth_cookie)
            .append_header(("location", service_url))
            .finish();

        return response;
    }

    log::warn!("Unauthorized Request detected.");

    HttpResponse::Unauthorized().body("Unauthorized")
}

/// Registers the current Schloss instance with the Schluessel entrypoint
async fn register_instance() -> Result<(), reqwest::Error> {
    let schluessel_endpoint = env::var("SCHLUESSEL_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:8080/register".to_string());

    log::info!(
        "Attempting to register Domain and Services with Schluessel at {schluessel_endpoint}"
    );

    let domain = env::var("DOMAIN").unwrap_or_else(|_| "127.0.0.1".to_string());
    let services = env::var("SERVICES").unwrap_or_else(|_| {
        json!([
            ["Service1", "http://127.0.0.1:8081"],
            ["Service2", "http://127.0.0.1:8082"]
        ])
        .to_string()
    });

    log::info!("Domain: {domain}");
    log::info!("Services: {services}");

    let data = json!({
        "domain": domain,
        "services": serde_json::from_str::<Vec<Vec<String>>>(&services).unwrap_or_default(),
    });

    let client = reqwest::Client::new();
    client.post(&schluessel_endpoint).json(&data).send().await?;

    Ok(())
}

/// Starts the server
#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // initialize logging
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    // read all the env vars
    let http_host = env::var("HTTP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let http_port = env::var("HTTP_PORT").unwrap_or_else(|_| "8081".to_string());

    let bind_address = format!("{}:{}", http_host, http_port);

    // set the schloss version
    let schloss_version = env::var("SCHLOSS_VERSION")
        .or_else(|_| env::var("CARGO_PKG_VERSION"))
        .unwrap_or_else(|_| "0.0.0-dev (not set)".to_string());

    // print out some basic info about the server
    log::info!("Starting Schloss v{schloss_version}");
    log::info!("Serving at {http_host}:{http_port}");

    if let Err(e) = register_instance().await {
        log::error!("Failed to register instance: {e}");
        log::info!("Exiting Schloss");
        process::exit(1);
    }

    // start server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .service(index)
            .service(generate_auth_cookie)
    })
    .bind(bind_address)?
    .run()
    .await
}