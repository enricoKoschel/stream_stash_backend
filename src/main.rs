mod v1router;

use rocket::fairing::Fairing;
use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::outcome::IntoOutcome;
use rocket::request::{FromRequest, Outcome};
use rocket::time::OffsetDateTime;
use rocket::{async_trait, launch, Request};
use rocket_cors::{AllowedHeaders, AllowedMethods, AllowedOrigins, CorsOptions};
use serde::{Deserialize, Serialize};
use std::time::Duration;

macro_rules! serde_struct {
    ($struct_name:ident, $($field_name:ident: $field_type:ty = $field_default:expr),+ $(,)?) => {
        #[derive(serde::Deserialize, serde::Serialize, Debug)]
        #[serde(default)]
        struct $struct_name {
            $(
                $field_name: $field_type,
            )*
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self {
                    $(
                        $field_name: $field_default,
                    )*
                }
            }
        }
    };
    ($struct_name:ident, $($field_name:ident: $field_type:ty),+ $(,)?) => {
        #[derive(serde::Deserialize, serde::Serialize, Debug)]
        struct $struct_name {
            $(
                $field_name: $field_type,
            )*
        }
    };
}

pub(crate) use serde_struct;

const FRONTEND_URL: &str = if cfg!(debug_assertions) {
    "http://localhost:9000"
} else {
    "https://stream-stash.com"
};

const SESSION_COOKIE_NAME: &str = "session";

const SESSION_COOKIE_DOMAIN: &str = if cfg!(debug_assertions) {
    "localhost"
} else {
    "stream-stash.com"
};

const SESSION_COOKIE_PATH: &str = "/";

// Disable secure cookie in development, some browsers don't support secure cookies on http://localhost
const SESSION_COOKIE_SECURE: bool = !cfg!(debug_assertions);

const SESSION_COOKIE_SAME_SITE: SameSite = SameSite::Strict;

const SESSION_COOKIE_HTTP_ONLY: bool = true;

fn session_cookie_expires() -> OffsetDateTime {
    // 2678400 seconds = 31 days
    OffsetDateTime::now_utc() + Duration::from_secs(2678400)
}

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

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum Session {
    LoggedIn(LoggedInSession),
    TempCodeVerifier(TempCodeVerifierSession),
}

#[derive(Serialize, Deserialize)]
struct LoggedInSession {
    access_token: String,
    refresh_token: String,
}

#[derive(Serialize, Deserialize)]
struct TempCodeVerifierSession {
    code_verifier: String,
}

#[async_trait]
impl<'a> FromRequest<'a> for LoggedInSession {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'a Request<'_>) -> Outcome<LoggedInSession, Self::Error> {
        request
            .cookies()
            .get_private("session")
            .and_then(|cookie| serde_json::from_str::<LoggedInSession>(cookie.value()).ok())
            .or_forward(Status::Forbidden)
    }
}

#[async_trait]
impl<'a> FromRequest<'a> for TempCodeVerifierSession {
    type Error = std::convert::Infallible;

    async fn from_request(
        request: &'a Request<'_>,
    ) -> Outcome<TempCodeVerifierSession, Self::Error> {
        request
            .cookies()
            .get_private("session")
            .and_then(|cookie| serde_json::from_str::<TempCodeVerifierSession>(cookie.value()).ok())
            .or_forward(Status::Forbidden)
    }
}

#[must_use]
fn add_session_cookie(jar: &CookieJar, session: &Session) -> bool {
    let Ok(session_string) = serde_json::to_string(session) else {
        return false;
    };

    let cookie = Cookie::build((SESSION_COOKIE_NAME, session_string))
        .domain(SESSION_COOKIE_DOMAIN)
        .path(SESSION_COOKIE_PATH)
        .secure(SESSION_COOKIE_SECURE)
        .same_site(SESSION_COOKIE_SAME_SITE)
        .http_only(SESSION_COOKIE_HTTP_ONLY)
        .expires(session_cookie_expires())
        .build();

    jar.add_private(cookie);

    true
}

fn remove_session_cookie(jar: &CookieJar) {
    let cookie = Cookie::build(SESSION_COOKIE_NAME)
        .domain(SESSION_COOKIE_DOMAIN)
        .path(SESSION_COOKIE_PATH)
        .secure(SESSION_COOKIE_SECURE)
        .same_site(SESSION_COOKIE_SAME_SITE)
        .http_only(SESSION_COOKIE_HTTP_ONLY)
        .removal()
        .build();

    jar.remove_private(cookie);
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

    let state = GoogleApplicationDetails {
        client_id: std::env::var("GOOGLE_CLIENT_ID")
            .expect("Please provide a GOOGLE_CLIENT_ID envvar"),
        client_secret: std::env::var("GOOGLE_CLIENT_SECRET")
            .expect("Please provide a GOOGLE_CLIENT_SECRET envvar"),
    };

    rocket::build()
        .attach(cors_fairing())
        .manage(state)
        .manage(reqwest::Client::new())
        .mount("/v1", v1router::routes())
}
