use crate::macros::{
    add_session_cookie, forbidden, get_json_body, internal_server_error, log_error_location,
    log_info_location, parse_url, serde_struct,
};
use crate::v1router::GOOGLE_SCOPE;
use crate::GoogleApplicationDetails;
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
                    match refresh_google_login(request, logged_in_session.refresh_token).await {
                        Ok(session) => Outcome::Success(session),
                        Err(_) => {
                            log_error_location!("Could not refresh Google login");

                            Outcome::Forward(Status::Forbidden)
                        }
                    }
                } else {
                    Outcome::Success(logged_in_session)
                }
            }
            Err(err) => {
                log_error_location!(
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
                log_error_location!(
                    "Trying to access {}, could not parse TempCodeVerifierSession from session cookie: {}",
                    request.uri().path(),
                    err
                );
                return Outcome::Forward(Status::Forbidden);
            }
        }
    }
}

async fn refresh_google_login(
    request: &Request<'_>,
    refresh_token: String,
) -> Result<LoggedInSession, crate::v1router::ApiError> {
    let rocket = request.rocket();
    let jar = request.cookies();
    let google_application_details = rocket.state::<GoogleApplicationDetails>().unwrap();
    let http_client = rocket.state::<reqwest::Client>().unwrap();

    serde_struct!(GoogleRequest,
        client_id: String,
        client_secret: String,
        grant_type: String,
        refresh_token: String,
    );
    serde_struct!(GoogleResponse,
        access_token:String,
        scope: String,
        token_type: String,
        expires_in: u64,
        id_token: String,
    );

    let token_url = parse_url!("https://oauth2.googleapis.com/token");
    let request = http_client.post(token_url).json(&GoogleRequest {
        client_id: google_application_details.client_id.clone(),
        client_secret: google_application_details.client_secret.clone(),
        grant_type: "refresh_token".to_string(),
        refresh_token: refresh_token.clone(),
    });
    let response = get_json_body!(request, GoogleResponse);

    match response {
        Ok(GoogleResponse {
            access_token,
            scope,
            token_type: _token_type,
            expires_in,
            id_token: _id_token,
        }) => {
            let requested_scope: std::collections::HashSet<&str> =
                GOOGLE_SCOPE.split_whitespace().collect();
            let received_scope: std::collections::HashSet<&str> =
                scope.split_whitespace().collect();

            if requested_scope != received_scope {
                return Err(forbidden!(
                    "Scope returned by google ({scope}) not the same as requested ({GOOGLE_SCOPE})"
                ));
            }

            let refreshed_session = LoggedInSession {
                access_token,
                refresh_token,
                expires_at: expires_at(expires_in),
            };

            add_session_cookie!(jar, Session::LoggedIn(refreshed_session.clone()));

            log_info_location!("Google login refreshed");

            Ok(refreshed_session)
        }
        Err(err) => Err(forbidden!("Could not reauthenticate with Google: {err}")),
    }
}

fn get_session_cookie<'a>(request: &'a Request) -> Option<Cookie<'a>> {
    match request.cookies().get_private(COOKIE_NAME) {
        Some(cookie) => Some(cookie),
        None => {
            log_error_location!(
                "Trying to access {}, could not get session cookie",
                request.uri().path()
            );
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

pub fn expires_at(expires_in: u64) -> i64 {
    const TOLERANCE: u64 = 100;

    (OffsetDateTime::now_utc() + Duration::from_secs(expires_in - TOLERANCE)).unix_timestamp()
}
