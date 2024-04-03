mod macros;
mod session;
mod v1router;

use rocket::fairing::Fairing;
use rocket::launch;
use rocket_cors::{AllowedHeaders, AllowedMethods, AllowedOrigins, CorsOptions};

const FRONTEND_URL: &str = if cfg!(debug_assertions) {
    "http://localhost:9000"
} else {
    "https://stream-stash.com"
};

fn cors_fairing() -> impl Fairing {
    let allowed_methods: AllowedMethods = ["Get", "Post", "Delete"]
        .iter()
        .map(|s| std::str::FromStr::from_str(s).unwrap())
        .collect();
    let allowed_headers = AllowedHeaders::some(&["content-type"]);
    let allowed_origins = AllowedOrigins::some_exact(&[FRONTEND_URL]);

    CorsOptions::default()
        .allowed_methods(allowed_methods)
        .allowed_headers(allowed_headers)
        .allowed_origins(allowed_origins)
        .allow_credentials(true)
        .to_cors()
        .unwrap()
}

struct GoogleApplicationDetails {
    client_id: String,
    client_secret: String,
}

#[launch]
fn rocket() -> _ {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let google_application_details = GoogleApplicationDetails {
        client_id: std::env::var("GOOGLE_CLIENT_ID")
            .expect("Please provide a GOOGLE_CLIENT_ID envvar"),
        client_secret: std::env::var("GOOGLE_CLIENT_SECRET")
            .expect("Please provide a GOOGLE_CLIENT_SECRET envvar"),
    };

    rocket::build()
        .attach(cors_fairing())
        .manage(google_application_details)
        .manage(reqwest::Client::new())
        .mount("/v1", v1router::routes())
}
