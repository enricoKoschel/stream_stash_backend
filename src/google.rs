use url::Url;

use crate::{
    expires_at,
    macros::{
        compare_scope, forbidden, get_json_body, log_error_location, parse_url, serde_struct,
    },
    session::LoggedInSession,
    ApiResult,
};

//TODO: Only request one scope at a time or send back response with header "returned google scopes not the same as requested" and show corresponding error in frontend
pub const AUTH_SCOPE: &str =
    "https://www.googleapis.com/auth/drive.appdata https://www.googleapis.com/auth/userinfo.email openid";

pub struct ApplicationDetails {
    pub client_id: String,
    pub client_secret: String,
}

fn compare_scope(scope: String) -> bool {
    let requested_scope: std::collections::HashSet<&str> = AUTH_SCOPE.split_whitespace().collect();
    let received_scope: std::collections::HashSet<&str> = scope.split_whitespace().collect();

    if requested_scope == received_scope {
        true
    } else {
        log_error_location!("Scope returned by google ({received_scope:#?}) not the same as requested ({requested_scope:#?})");
        false
    }
}

pub fn generate_auth_url_and_code_verifier(
    google_application_details: &ApplicationDetails,
) -> ApiResult<(Url, String)> {
    let code_verifier = pkce::code_verifier(128);
    let code_challenge = pkce::code_challenge(&code_verifier);

    let mut auth_url = parse_url!("https://accounts.google.com/o/oauth2/v2/auth")?;
    auth_url
        .query_pairs_mut()
        .append_pair("client_id", &google_application_details.client_id)
        .append_pair("redirect_uri", crate::frontend::REDIRECT_URL)
        .append_pair("response_type", "code")
        .append_pair("prompt", "consent")
        .append_pair("access_type", "offline")
        .append_pair("scope", AUTH_SCOPE)
        .append_pair("code_challenge", &code_challenge)
        .append_pair("code_challenge_method", "S256");

    let code_verifier_string: String = code_verifier.iter().map(|n| *n as char).collect();

    Ok((auth_url, code_verifier_string))
}

pub async fn create_session(
    google_application_details: &ApplicationDetails,
    http_client: &reqwest::Client,
    auth_code: String,
    code_verifier: String,
) -> ApiResult<LoggedInSession> {
    serde_struct!(GoogleRequest,
        client_id: String,
        client_secret: String,
        code: String,
        code_verifier: String,
        grant_type: String,
        redirect_uri: String,
    );
    serde_struct!(GoogleResponse,
        access_token:String,
        scope: String,
        token_type: String,
        expires_in: u64,
        refresh_token: String,
    );

    let token_url = parse_url!("https://oauth2.googleapis.com/token")?;
    let request = http_client.post(token_url).json(&GoogleRequest {
        client_id: google_application_details.client_id.clone(),
        client_secret: google_application_details.client_secret.clone(),
        code: auth_code,
        code_verifier,
        grant_type: "authorization_code".to_string(),
        redirect_uri: crate::frontend::REDIRECT_URL.to_string(),
    });
    let response = get_json_body!(request, GoogleResponse)?;

    match response {
        Ok(GoogleResponse {
            access_token,
            scope,
            token_type: _,
            expires_in,
            refresh_token,
        }) => {
            compare_scope!(scope)?;

            Ok(LoggedInSession {
                access_token,
                refresh_token,
                expires_at: expires_at(expires_in),
            })
        }
        Err(err) => Err(forbidden!("Could not authenticate with Google: {err}")),
    }
}

pub async fn revoke_session(
    http_client: &reqwest::Client,
    session: &LoggedInSession,
) -> ApiResult<()> {
    let mut revoke_url = parse_url!("https://oauth2.googleapis.com/revoke")?;
    revoke_url
        .query_pairs_mut()
        .append_pair("token", &session.access_token);

    let request = http_client.post(revoke_url);
    // The response data sent by Google is irrelevant, only the status matters
    let _ = get_json_body!(request, serde_json::Value)?;

    Ok(())
}

pub async fn get_user_email(
    http_client: &reqwest::Client,
    session: &LoggedInSession,
) -> ApiResult<String> {
    serde_struct!(GoogleResponse,
        picture: String,
        id: String,
        email: String,
        verified_email: bool,
    );

    let api_url = parse_url!("https://www.googleapis.com/oauth2/v2/userinfo")?;
    let request = http_client
        .get(api_url)
        .header("authorization", format!("Bearer {}", session.access_token)); // TODO: add function to RequestBuild which adds the authorization header
    let response = get_json_body!(request, GoogleResponse)?;

    match response {
        Ok(GoogleResponse {
            picture: _,
            id: _,
            email,
            verified_email: _,
        }) => Ok(email),
        Err(err) => Err(forbidden!(
            "Could not get user information from Google: {err}"
        )),
    }
}

pub async fn create_refreshed_session(
    google_application_details: &ApplicationDetails,
    http_client: &reqwest::Client,
    refresh_token: String,
) -> ApiResult<LoggedInSession> {
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

    let token_url = parse_url!("https://oauth2.googleapis.com/token")?;
    let request = http_client.post(token_url).json(&GoogleRequest {
        client_id: google_application_details.client_id.clone(),
        client_secret: google_application_details.client_secret.clone(),
        grant_type: "refresh_token".to_string(),
        refresh_token: refresh_token.clone(),
    });
    let response = get_json_body!(request, GoogleResponse)?;

    match response {
        Ok(GoogleResponse {
            access_token,
            scope,
            token_type: _,
            expires_in,
            id_token: _,
        }) => {
            compare_scope!(scope)?;

            let refreshed_session = LoggedInSession {
                access_token,
                refresh_token,
                expires_at: expires_at(expires_in),
            };

            Ok(refreshed_session)
        }
        Err(err) => Err(forbidden!("Could not reauthenticate with Google: {err}")),
    }
}
