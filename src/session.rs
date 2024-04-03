use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::request::{FromRequest, Outcome};
use rocket::serde::{Deserialize, Serialize};
use rocket::time::OffsetDateTime;
use rocket::{async_trait, Request};
use std::time::Duration;

const COOKIE_NAME: &str = "session";

const COOKIE_DOMAIN: &str = if cfg!(debug_assertions) {
    "localhost"
} else {
    "stream-stash.com"
};

const COOKIE_PATH: &str = "/";

// Disable secure cookie in development, some browsers don't support secure cookies on http://localhost
const COOKIE_SECURE: bool = !cfg!(debug_assertions);

const COOKIE_SAME_SITE: SameSite = SameSite::Strict;

const COOKIE_HTTP_ONLY: bool = true;

fn cookie_expires() -> OffsetDateTime {
    // 2678400 seconds = 31 days
    OffsetDateTime::now_utc() + Duration::from_secs(2678400)
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Session {
    LoggedIn(LoggedInSession),
    TempCodeVerifier(TempCodeVerifierSession),
}

#[derive(Serialize, Deserialize)]
pub struct LoggedInSession {
    pub(crate) access_token: String,
    pub(crate) refresh_token: String,
}

#[derive(Serialize, Deserialize)]
pub struct TempCodeVerifierSession {
    pub(crate) code_verifier: String,
}

#[async_trait]
impl<'a> FromRequest<'a> for LoggedInSession {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'a Request<'_>) -> Outcome<LoggedInSession, Self::Error> {
        let Some(session_cookie) = get_session_cookie(request) else {
            return Outcome::Forward(Status::Forbidden);
        };

        match serde_json::from_str::<LoggedInSession>(session_cookie.value()) {
            Ok(logged_in_session) => Outcome::Success(logged_in_session),
            Err(err) => {
                log::error!(
                    "Trying to access {}, could not parse LoggedInSession from session cookie: {}",
                    request.uri().path(),
                    err
                );
                return Outcome::Forward(Status::Forbidden);
            }
        }
    }
}

#[async_trait]
impl<'a> FromRequest<'a> for TempCodeVerifierSession {
    type Error = std::convert::Infallible;

    async fn from_request(
        request: &'a Request<'_>,
    ) -> Outcome<TempCodeVerifierSession, Self::Error> {
        let Some(session_cookie) = get_session_cookie(request) else {
            return Outcome::Forward(Status::Forbidden);
        };

        match serde_json::from_str::<TempCodeVerifierSession>(session_cookie.value()) {
            Ok(temp_code_verifier_session) => Outcome::Success(temp_code_verifier_session),
            Err(err) => {
                log::error!(
                    "Trying to access {}, could not parse TempCodeVerifier from session cookie: {}",
                    request.uri().path(),
                    err
                );
                return Outcome::Forward(Status::Forbidden);
            }
        }
    }
}

fn get_session_cookie<'a>(request: &'a Request) -> Option<Cookie<'a>> {
    match request.cookies().get_private(COOKIE_NAME) {
        Some(cookie) => Some(cookie),
        None => {
            log::error!(
                "Trying to access {}, could not get session cookie",
                request.uri().path()
            );
            None
        }
    }
}

#[must_use]
pub fn add_session_cookie(jar: &CookieJar, session: &Session) -> bool {
    let Ok(session_string) = serde_json::to_string(session) else {
        return false;
    };

    let cookie = Cookie::build((COOKIE_NAME, session_string))
        .domain(COOKIE_DOMAIN)
        .path(COOKIE_PATH)
        .secure(COOKIE_SECURE)
        .same_site(COOKIE_SAME_SITE)
        .http_only(COOKIE_HTTP_ONLY)
        .expires(cookie_expires())
        .build();

    jar.add_private(cookie);

    true
}

pub fn remove_session_cookie(jar: &CookieJar) {
    let cookie = Cookie::build(COOKIE_NAME)
        .domain(COOKIE_DOMAIN)
        .path(COOKIE_PATH)
        .secure(COOKIE_SECURE)
        .same_site(COOKIE_SAME_SITE)
        .http_only(COOKIE_HTTP_ONLY)
        .removal()
        .build();

    jar.remove_private(cookie);
}
