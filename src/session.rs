use crate::google::{create_refreshed_session, ApplicationDetails};
use crate::macros::{add_session_cookie, log_error_location};
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Session {
    LoggedIn(LoggedInSession),
    TempCodeVerifier(TempCodeVerifierSession),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoggedInSession {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TempCodeVerifierSession {
    pub code_verifier: String,
}

#[async_trait]
impl<'a> FromRequest<'a> for LoggedInSession {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'a Request<'_>) -> Outcome<LoggedInSession, Self::Error> {
        let Some(session_cookie) = get_session_cookie(request) else {
            return Outcome::Forward(Status::Forbidden);
        };

        match serde_json::from_str::<LoggedInSession>(session_cookie.value()) {
            Ok(logged_in_session) => {
                if OffsetDateTime::now_utc().unix_timestamp() >= logged_in_session.expires_at {
                    let jar = request.cookies();
                    let rocket = request.rocket();
                    let google_application_details = rocket.state::<ApplicationDetails>().unwrap();
                    let http_client = rocket.state::<reqwest::Client>().unwrap();

                    match create_refreshed_session(
                        google_application_details,
                        http_client,
                        logged_in_session.refresh_token,
                    )
                    .await
                    {
                        Ok(session) => {
                            match add_session_cookie!(jar, &Session::LoggedIn(session.clone())) {
                                Ok(()) => Outcome::Success(session),
                                Err(err) => {
                                    log::error!("{err}");
                                    Outcome::Forward(Status::Forbidden)
                                }
                            }
                        }
                        Err(err) => {
                            log_error_location!("Could not refresh Google login: {err}");

                            Outcome::Forward(Status::Forbidden)
                        }
                    }
                } else {
                    Outcome::Success(logged_in_session)
                }
            }
            Err(err) => {
                log_error_location!("Could not parse LoggedInSession from session cookie: {err}");
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
                log_error_location!(
                    "Could not parse TempCodeVerifierSession from session cookie: {err}"
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
            log_error_location!("Could not get session cookie");
            None
        }
    }
}

#[must_use]
pub fn add_session_cookie(jar: &CookieJar, session: &Session) -> bool {
    let session_string = match serde_json::to_string(session) {
        Ok(session_string) => session_string,
        Err(err) => {
            log_error_location!("JSON serialize error: {err}");
            return false;
        }
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
