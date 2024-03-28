use crate::serde_struct;
use tide::{Response, StatusCode};

serde_struct!(LoginReqBody, access_token: String);
serde_struct!(LoginResBody, success: bool);

/*
Request query: <empty>

Request body: {
    access_token: String,
}

Response body: {
    success: bool,
}
*/
async fn login(mut req: tide::Request<()>) -> tide::Result {
    let Ok(_login_body): tide::Result<LoginReqBody> = req.body_json().await else {
        return Ok(Response::new(StatusCode::BadRequest));
    };

    // TODO: Validate access_token with google api and insert into session

    /*req.session_mut()
    .insert("access_token", login_body.access_token)?;*/

    let res = Response::builder(StatusCode::Ok)
        .body_json(&LoginResBody { success: false })?
        .build();

    Ok(res)
}

/*
Request query: <empty>

Request body: <empty>

Response body: <empty>
*/
async fn logout(mut req: tide::Request<()>) -> tide::Result {
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
async fn user_info(req: tide::Request<()>) -> tide::Result {
    let access_token: Option<String> = req.session().get("access_token");

    match access_token {
        Some(_access_token) => {
            // TODO: Get username from google api
            let username = "".to_string();

            let res = Response::builder(StatusCode::Ok)
                .body_json(&UserInfoResBody {
                    logged_in: true,
                    username: Some(username),
                })?
                .build();

            Ok(res)
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

pub fn new() -> tide::Server<()> {
    let mut server = tide::new();

    server.at("/login").post(login);
    server.at("/logout").delete(logout);
    server.at("/userInfo").get(user_info);

    server
}
