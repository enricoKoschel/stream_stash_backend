use crate::macros::{
    add_session_cookie, forbidden, get_json_body, internal_server_error, log_error_location,
    parse_url, serde_enum, serde_struct,
};
use crate::session::{remove_session_cookie, LoggedInSession, Session, TempCodeVerifierSession};
use crate::{GoogleApplicationDetails, FRONTEND_URL};
use const_format::concatcp;
use rocket::http::{CookieJar, Status};
use rocket::serde::json::Json;
use rocket::{delete, get, post, Responder, State};

// TODO: Update readme

const REDIRECT_URI: &str = concatcp!(FRONTEND_URL, "/loginRedirect");
const GOOGLE_SCOPE: &str =
    "https://www.googleapis.com/auth/drive.appdata https://www.googleapis.com/auth/userinfo.profile";

#[derive(Responder)]
enum ApiError {
    #[response(status = 403)]
    Forbidden(()),
    #[response(status = 500)]
    InternalServerError(()),
}

/*
--- /v1/googleLogin ---

Request query: <empty>

Request body: <empty>

Response body: {
    google_auth_url: String,
}
*/
serde_struct!(GoogleLoginResBody, google_auth_url: String);

#[get("/googleLogin")]
fn google_login(
    jar: &CookieJar,
    google_application_details: &State<GoogleApplicationDetails>,
) -> Result<Json<GoogleLoginResBody>, ApiError> {
    let code_verifier = pkce::code_verifier(128);
    let code_challenge = pkce::code_challenge(&code_verifier);

    let mut google_auth_url = parse_url!("https://accounts.google.com/o/oauth2/v2/auth");
    google_auth_url
        .query_pairs_mut()
        .append_pair("client_id", &google_application_details.client_id)
        .append_pair("redirect_uri", REDIRECT_URI)
        .append_pair("response_type", "code")
        .append_pair("prompt", "consent")
        .append_pair("access_type", "offline")
        .append_pair("scope", GOOGLE_SCOPE)
        .append_pair("code_challenge", &code_challenge)
        .append_pair("code_challenge_method", "S256");

    let code_verifier_string: String = code_verifier.iter().map(|n| *n as char).collect();
    add_session_cookie!(
        jar,
        Session::TempCodeVerifier(TempCodeVerifierSession {
            code_verifier: code_verifier_string,
        })
    );

    Ok(Json(GoogleLoginResBody {
        google_auth_url: google_auth_url.to_string(),
    }))
}

/*
--- /v1/finishLogin ---

Request query: <empty>

Request body: {
    code: String,
}

Response body: <empty>
*/
serde_struct!(FinishLoginReqBody, code: String);

#[post("/finishLogin", format = "json", data = "<req_body>")]
async fn finish_login(
    jar: &CookieJar<'_>,
    session: TempCodeVerifierSession,
    google_application_details: &State<GoogleApplicationDetails>,
    http_client: &State<reqwest::Client>,
    req_body: Json<FinishLoginReqBody>,
) -> Result<Status, ApiError> {
    serde_struct!(GoogleRequest,
        client_id: String,
        client_secret: String,
        code: String,
        code_verifier: String,
        grant_type: String,
        redirect_uri: String,
    );
    serde_struct!(GoogleOkResponse,
        access_token:String,
        scope: String,
        token_type: String,
        expires_in: u64,
        refresh_token: String,
    );
    serde_struct!(GoogleErrResponse,
        error_description: String,
        error: String,
    );
    serde_enum!(GoogleResponse, Ok(GoogleOkResponse), Err(GoogleErrResponse));

    let token_url = parse_url!("https://oauth2.googleapis.com/token");
    let request = http_client.post(token_url).json(&GoogleRequest {
        client_id: google_application_details.client_id.clone(),
        client_secret: google_application_details.client_secret.clone(),
        code: req_body.code.clone(),
        code_verifier: session.code_verifier,
        grant_type: "authorization_code".to_string(),
        redirect_uri: REDIRECT_URI.to_string(),
    });
    let response = get_json_body!(request, GoogleResponse);

    match response {
        GoogleResponse::Ok(GoogleOkResponse {
            access_token,
            scope,
            token_type: _token_type,
            expires_in: _expires_in,
            refresh_token,
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

            add_session_cookie!(
                jar,
                Session::LoggedIn(LoggedInSession {
                    access_token,
                    refresh_token,
                })
            );

            Ok(Status::Ok)
        }
        GoogleResponse::Err(GoogleErrResponse {
            error_description,
            error,
        }) => Err(forbidden!(
            "Could not authenticate with google: {error} - {error_description}"
        )),
    }
}

/*
--- /v1/logout ---

Request query: <empty>

Request body: <empty>

Response body: <empty>
*/
#[delete("/logout")]
async fn logout(jar: &CookieJar<'_>) -> Status {
    remove_session_cookie(jar);

    Status::Ok
}

/*
--- /v1/userInfo ---

Request query: <empty>

Request body: <empty>

Response body: {
    logged_in: bool,
    username: String,
}
*/
serde_struct!(UserInfoResBody, logged_in: bool, username: Option<String>);

#[get("/userInfo")]
async fn user_info(
    session: Option<LoggedInSession>,
    http_client: &State<reqwest::Client>,
) -> Result<Json<UserInfoResBody>, ApiError> {
    match session {
        Some(session) => {
            // TODO: Check for expired access_token

            serde_struct!(GoogleOkResponse,
                locale: String,
                given_name: String,
                picture: String,
                id: String,
                name: String,
            );
            // TODO: Check this
            serde_struct!(GoogleErrResponse,
                error_description: String,
                error: String,
            );
            serde_enum!(GoogleResponse, Ok(GoogleOkResponse), Err(GoogleErrResponse));

            let api_url = parse_url!("https://www.googleapis.com/oauth2/v2/userinfo");
            let request = http_client
                .get(api_url)
                .header("authorization", format!("Bearer {}", session.access_token));
            let response = get_json_body!(request, GoogleResponse);

            match response {
                GoogleResponse::Ok(GoogleOkResponse {
                    locale: _locale,
                    given_name: _given_name,
                    picture: _picture,
                    id: _id,
                    name,
                }) => Ok(Json(UserInfoResBody {
                    logged_in: true,
                    username: Some(name),
                })),
                GoogleResponse::Err(GoogleErrResponse {
                    error_description,
                    error,
                }) => Err(forbidden!(
                    "Could not get user information: {error} - {error_description}"
                )),
            }
        }
        None => Ok(Json(UserInfoResBody {
            logged_in: false,
            username: None,
        })),
    }
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![google_login, finish_login, logout, user_info]
}
