mod frontend;
mod google;
mod macros;
mod session;
mod tmdb;
mod v1router;

use google::ApplicationDetails;
use rocket::launch;
use rocket::{fairing::Fairing, time::OffsetDateTime};
use rocket_cors::{AllowedHeaders, AllowedMethods, AllowedOrigins, CorsOptions};
use std::time::Duration;
use tmdb::ReadAccessToken;

#[derive(Debug)]
struct ErrorContext {
    file: &'static str,
    line: u32,
    column: u32,
}

impl std::fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}:{}", self.file, self.line, self.column)
    }
}

#[derive(Debug)]
enum ApiError {
    Forbidden(String, ErrorContext),
    InternalServerError(String, ErrorContext),
}

impl<'r, 'o: 'r> rocket::response::Responder<'r, 'o> for ApiError {
    fn respond_to(self, request: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        log::error!("{self}");

        match self {
            ApiError::Forbidden(_, _) => rocket::http::Status::Forbidden,
            ApiError::InternalServerError(_, _) => rocket::http::Status::InternalServerError,
        }
        .respond_to(request)
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Forbidden(msg, ctx) => {
                write!(f, "[{}] (FORBIDDEN) {}", ctx, msg)
            }
            ApiError::InternalServerError(msg, ctx) => {
                write!(f, "[{}] (INTERNAL SERVER ERROR) {}", ctx, msg)
            }
        }
    }
}

impl std::error::Error for ApiError {}

type ApiResult<T> = Result<T, ApiError>;

pub fn expires_at(expires_in: u64) -> i64 {
    const TOLERANCE: u64 = 100;

    (OffsetDateTime::now_utc() + Duration::from_secs(expires_in - TOLERANCE)).unix_timestamp()
}

fn cors_fairing() -> impl Fairing {
    let allowed_methods: AllowedMethods = ["Get", "Post", "Delete"]
        .iter()
        .map(|s| std::str::FromStr::from_str(s).unwrap())
        .collect();
    let allowed_headers = AllowedHeaders::some(&["content-type"]);
    let allowed_origins = AllowedOrigins::some_exact(frontend::BASE_URLS);

    CorsOptions::default()
        .allowed_methods(allowed_methods)
        .allowed_headers(allowed_headers)
        .allowed_origins(allowed_origins)
        .allow_credentials(true)
        .to_cors()
        .unwrap()
}

#[launch]
fn rocket() -> _ {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info) // TODO: Put in envvar locally and on server
        .init();

    let google_application_details = ApplicationDetails {
        client_id: std::env::var("GOOGLE_CLIENT_ID")
            .expect("Please provide a GOOGLE_CLIENT_ID envvar"),
        client_secret: std::env::var("GOOGLE_CLIENT_SECRET")
            .expect("Please provide a GOOGLE_CLIENT_SECRET envvar"),
    };

    let tmdb_read_access_token = std::env::var("TMDB_READ_ACCESS_TOKEN")
        .expect("Please provide a TMDB_READ_ACCESS_TOKEN envvar");

    rocket::build()
        .attach(cors_fairing())
        .manage(google_application_details)
        .manage(reqwest::Client::new())
        .manage(ReadAccessToken(tmdb_read_access_token))
        .mount("/v1", v1router::routes())
}
