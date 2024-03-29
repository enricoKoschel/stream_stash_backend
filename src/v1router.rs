use crate::{serde_struct, FRONTEND_URL};
use const_format::concatcp;
use serde::Deserialize;
use tide::{Response, StatusCode};

// TODO: Add logging
// TODO: Update readme

const REDIRECT_URI: &str = concatcp!(FRONTEND_URL, "/loginRedirect");
const GOOGLE_SCOPE: &str =
    "https://www.googleapis.com/auth/drive.appdata https://www.googleapis.com/auth/userinfo.profile";

serde_struct!(GoogleLoginResBody, google_auth_url: String);

/*
Request query: <empty>

Request body: <empty>

Response body: {
    google_auth_url: String,
}
*/
async fn google_login(mut req: tide::Request<State>) -> tide::Result {
    let code_verifier = pkce::code_verifier(128);
    let code_challenge = pkce::code_challenge(&code_verifier);

    let mut google_auth_url = surf::Url::parse("https://accounts.google.com/o/oauth2/v2/auth")?;
    google_auth_url
        .query_pairs_mut()
        .append_pair("client_id", &req.state().google_client_id);
    google_auth_url
        .query_pairs_mut()
        .append_pair("redirect_uri", REDIRECT_URI);
    google_auth_url
        .query_pairs_mut()
        .append_pair("response_type", "code");
    google_auth_url
        .query_pairs_mut()
        .append_pair("prompt", "consent");
    google_auth_url
        .query_pairs_mut()
        .append_pair("access_type", "offline");
    google_auth_url
        .query_pairs_mut()
        .append_pair("scope", GOOGLE_SCOPE);
    google_auth_url
        .query_pairs_mut()
        .append_pair("code_challenge", &code_challenge);
    google_auth_url
        .query_pairs_mut()
        .append_pair("code_challenge_method", "S256");

    let code_verifier_string: String = code_verifier.iter().map(|n| *n as char).collect();
    req.session_mut()
        .insert("code_verifier", code_verifier_string)?;

    let res = Response::builder(StatusCode::Ok)
        .body_json(&GoogleLoginResBody {
            google_auth_url: google_auth_url.to_string(),
        })?
        .build();

    Ok(res)
}

serde_struct!(FinishLoginReqBody, code: String);
serde_struct!(FinishLoginResBody, success: bool);

/*
Request query: <empty>

Request body: {
    code: String,
}

Response body: <empty>
*/
async fn finish_login(mut req: tide::Request<State>) -> tide::Result {
    let Ok(req_body): tide::Result<FinishLoginReqBody> = req.body_json().await else {
        return Ok(Response::new(StatusCode::BadRequest));
    };

    let Some(code_verifier): Option<String> = req.session_mut().get("code_verifier") else {
        return Ok(Response::new(StatusCode::Forbidden));
    };

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

    // TODO: Macro for this?
    #[derive(Deserialize, Debug)]
    #[serde(untagged)]
    enum GoogleResponse {
        Ok(GoogleOkResponse),
        Err(GoogleErrResponse),
    }

    let token_url = surf::Url::parse("https://oauth2.googleapis.com/token")?;
    let google_response: GoogleResponse = req
        .state()
        .http_client
        .post(token_url)
        .body_json(&GoogleRequest {
            client_id: req.state().google_client_id.clone(),
            client_secret: req.state().google_client_secret.clone(),
            code: req_body.code,
            code_verifier,
            grant_type: "authorization_code".to_string(),
            redirect_uri: REDIRECT_URI.to_string(),
        })?
        .await?
        .body_json()
        .await?;

    match google_response {
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
                log::error!(
                    "Scope returned by google ({scope}) not the same as requested ({GOOGLE_SCOPE})"
                );
                return Ok(Response::new(StatusCode::Forbidden));
            }

            req.session_mut().insert("access_token", access_token)?;
            req.session_mut().insert("refresh_token", refresh_token)?;

            Ok(Response::new(StatusCode::Ok))
        }
        GoogleResponse::Err(GoogleErrResponse {
            error_description,
            error,
        }) => {
            log::error!("Could not authenticate with google: {error} - {error_description}");
            Ok(Response::new(StatusCode::Forbidden))
        }
    }
}

/*
Request query: <empty>

Request body: <empty>

Response body: <empty>
*/
async fn logout(mut req: tide::Request<State>) -> tide::Result {
    req.session_mut().destroy();

    let res = Response::new(StatusCode::Ok);

    Ok(res)
}

serde_struct!(UserInfoResBody, logged_in: bool, username: Option<String>);

/*
Request query: <empty>

Request body: <empty>

Response body: {
    logged_in: bool,
    username: String,
}
*/
async fn user_info(req: tide::Request<State>) -> tide::Result {
    // TODO: Check for expired token
    let access_token: Option<String> = req.session().get("access_token");

    match access_token {
        Some(access_token) => {
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

            // TODO: Macro for this?
            #[derive(Deserialize, Debug)]
            #[serde(untagged)]
            enum GoogleResponse {
                Ok(GoogleOkResponse),
                Err(GoogleErrResponse),
            }

            let api_url = surf::Url::parse("https://www.googleapis.com/oauth2/v2/userinfo")?;
            let google_response: GoogleResponse = req
                .state()
                .http_client
                .get(api_url)
                .header("authorization", format!("Bearer {access_token}"))
                .send()
                .await?
                .body_json()
                .await?;

            match google_response {
                GoogleResponse::Ok(GoogleOkResponse {
                    locale: _locale,
                    given_name: _given_name,
                    picture: _picture,
                    id: _id,
                    name,
                }) => {
                    let res = Response::builder(StatusCode::Ok)
                        .body_json(&UserInfoResBody {
                            logged_in: true,
                            username: Some(name),
                        })?
                        .build();

                    Ok(res)
                }
                GoogleResponse::Err(GoogleErrResponse {
                    error_description,
                    error,
                }) => {
                    log::error!("Could not get user information: {error} - {error_description}");
                    Ok(Response::new(StatusCode::Forbidden))
                }
            }
        }
        None => {
            let res = Response::builder(StatusCode::Ok)
                .body_json(&UserInfoResBody {
                    logged_in: false,
                    username: None,
                })?
                .build();

            Ok(res)
        }
    }
}

#[derive(Clone)]
pub struct State {
    google_client_id: String,
    google_client_secret: String,
    http_client: surf::Client,
}

pub fn new() -> tide::Server<State> {
    let mut server = tide::with_state(State {
        google_client_id: std::env::var("GOOGLE_CLIENT_ID")
            .expect("Please provide a GOOGLE_CLIENT_ID envvar"),
        google_client_secret: std::env::var("GOOGLE_CLIENT_SECRET")
            .expect("Please provide a GOOGLE_CLIENT_SECRET envvar"),
        http_client: surf::Client::new(),
    });

    server.at("/googleLogin").get(google_login);
    server.at("/finishLogin").post(finish_login);
    server.at("/logout").delete(logout);
    server.at("/userInfo").get(user_info);

    server
}
