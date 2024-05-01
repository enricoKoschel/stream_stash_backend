use crate::google::{
    create_session, generate_auth_url_and_code_verifier, get_and_update_db_file,
    get_db_file_contents, get_user_email, revoke_session,
};
use crate::macros::{add_session_cookie, serde_struct};
use crate::session::{remove_session_cookie, LoggedInSession, Session, TempCodeVerifierSession};
use crate::tmdb::{self, ReadAccessToken, SearchResult};
use crate::{ApiResult, ApplicationDetails};
use rocket::http::CookieJar;
use rocket::serde::json::Json;
use rocket::{delete, get, post, State};

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
    google_application_details: &State<ApplicationDetails>,
) -> ApiResult<Json<GoogleLoginResBody>> {
    let (auth_url, code_verifier) =
        generate_auth_url_and_code_verifier(google_application_details)?;

    add_session_cookie!(
        jar,
        Session::TempCodeVerifier(TempCodeVerifierSession { code_verifier })
    )?;

    Ok(Json(GoogleLoginResBody {
        google_auth_url: auth_url.to_string(),
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
    google_application_details: &State<ApplicationDetails>,
    http_client: &State<reqwest::Client>,
    req_body: Json<FinishLoginReqBody>,
) -> ApiResult<()> {
    let session = create_session(
        google_application_details,
        http_client,
        req_body.code.clone(),
        session.code_verifier,
    )
    .await?;

    add_session_cookie!(jar, Session::LoggedIn(session))?;

    Ok(())
}

/*
--- /v1/logout ---

Request query: <empty>

Request body: <empty>

Response body: <empty>
*/
#[delete("/logout")]
async fn logout(
    jar: &CookieJar<'_>,
    session: LoggedInSession,
    http_client: &State<reqwest::Client>,
) -> ApiResult<()> {
    revoke_session(http_client, &session).await?;

    remove_session_cookie(jar);

    Ok(())
}

/*
--- /v1/userInfo ---

Request query: <empty>

Request body: <empty>

Response body: {
    logged_in: bool,
    email: String,
}
*/
serde_struct!(UserInfoResBody, logged_in: bool, email: Option<String>);

#[get("/userInfo")]
async fn user_info(
    session: Option<LoggedInSession>,
    http_client: &State<reqwest::Client>,
) -> ApiResult<Json<UserInfoResBody>> {
    match session {
        Some(session) => {
            let email = get_user_email(http_client, &session).await?;

            Ok(Json(UserInfoResBody {
                logged_in: true,
                email: Some(email),
            }))
        }
        None => Ok(Json(UserInfoResBody {
            logged_in: false,
            email: None,
        })),
    }
}

/*
--- /v1/updateMedia ---

Request query: <empty>

Request body: {
    media: String
}

Response body: <empty>
*/
serde_struct!(UpdateMediaReqBody, media: String);

#[post("/updateMedia", format = "json", data = "<req_body>")]
async fn update_media(
    session: LoggedInSession,
    http_client: &State<reqwest::Client>,
    req_body: Json<UpdateMediaReqBody>,
) -> ApiResult<()> {
    get_and_update_db_file(http_client, &session, &req_body.media).await?;

    Ok(())
}

/*
--- /v1/getMedia ---

Request query: <empty>

Request body: <empty>

Response body: {
    media: String
}
*/
serde_struct!(GetMediaResBody, media: String);

#[get("/getMedia")]
async fn get_media(
    session: LoggedInSession,
    http_client: &State<reqwest::Client>,
) -> ApiResult<Json<GetMediaResBody>> {
    let media = get_db_file_contents(http_client, &session).await?;

    Ok(Json(GetMediaResBody { media }))
}

serde_struct!(SearchReqQuery, query: String, page: u32);

/*
--- /v1/movieSearch ---

Request query: {
    query: String,
    page: u32,
}

Request body: <empty

Response body: {
    page: u32,
    results: Vec<Media>,
    total_pages: u32,
    total_results: u32,
}
*/
#[get("/movieSearch?<query>&<page>")]
async fn movie_search(
    http_client: &State<reqwest::Client>,
    tmdb_read_access_token: &State<ReadAccessToken>,
    query: &str,
    page: u32,
) -> ApiResult<Json<SearchResult>> {
    let result = tmdb::movie_search(tmdb_read_access_token, http_client, query, page).await?;

    Ok(Json(result))
}

/*
--- /v1/tvSearch ---

Request query: {
    query: String,
    page: u32,
}

Request body: <empty>

Response body: {
    page: u32,
    results: Vec<Media>,
    total_pages: u32,
    total_results: u32,
}
*/
#[get("/tvSearch?<query>&<page>")]
async fn tv_search(
    http_client: &State<reqwest::Client>,
    tmdb_read_access_token: &State<ReadAccessToken>,
    query: &str,
    page: u32,
) -> ApiResult<Json<SearchResult>> {
    let result = tmdb::tv_search(tmdb_read_access_token, http_client, query, page).await?;

    Ok(Json(result))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        google_login,
        finish_login,
        logout,
        user_info,
        update_media,
        get_media,
        movie_search,
        tv_search,
    ]
}
