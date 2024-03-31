use rocket::fairing::Fairing;
use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::outcome::IntoOutcome;
use rocket::request::{FromRequest, Outcome};
use rocket::response::Redirect;
use rocket::time::OffsetDateTime;
use rocket::{async_trait, get, launch, routes, Request};
use rocket_cors::{AllowedHeaders, AllowedMethods, AllowedOrigins, CorsOptions};
use std::time::Duration;

const FRONTEND_URL: &str = if cfg!(debug_assertions) {
    "http://localhost:9000"
} else {
    "https://stream-stash.com"
};

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

#[derive(serde::Serialize, serde::Deserialize)]
struct Session {
    username: String,
    age: u32,
}

#[async_trait]
impl<'r> FromRequest<'r> for Session {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Session, Self::Error> {
        request
            .cookies()
            .get_private("session")
            .and_then(|cookie| serde_json::from_str::<Session>(cookie.value()).ok())
            .or_forward(Status::Forbidden)
    }
}

#[get("/")]
fn index(session: Option<Session>) -> String {
    match session {
        Some(session) => {
            format!(
                "You are logged in as {}, {} year(s) old",
                session.username, session.age
            )
        }
        None => "You are not logged in".to_string(),
    }
}

#[get("/login/<username>/<age>")]
fn login(jar: &CookieJar<'_>, username: String, age: u32) -> Result<Redirect, Status> {
    let Ok(session_string) = serde_json::to_string(&Session { username, age }) else {
        return Err(Status::InternalServerError);
    };

    let cookie_domain = if cfg!(debug_assertions) {
        "localhost"
    } else {
        "stream-stash.com"
    };

    // Disable secure cookies in development, some browsers don't support secure cookies on http://localhost
    let secure = !cfg!(debug_assertions);

    // 2678400 seconds = 31 days
    let expires = OffsetDateTime::now_utc() + Duration::from_secs(2678400);

    let session_cookie = Cookie::build(("session", session_string))
        .domain(cookie_domain)
        .secure(secure)
        .same_site(SameSite::Strict)
        .http_only(true)
        .expires(expires);
    jar.add_private(session_cookie);

    Ok(Redirect::to("/"))
}

#[get("/logout")]
fn logout(jar: &CookieJar<'_>) -> Redirect {
    jar.remove_private("session");

    Redirect::to("/")
}

#[launch]
fn rocket() -> _ {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    rocket::build()
        .attach(cors_fairing())
        .mount("/", routes![index, login, logout])
}
