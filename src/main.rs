macro_rules! param_struct {
    ($struct_name:ident, $($field_name:ident: $field_type:ty = $field_default:expr),+) => {
        use crate::{Deserialize, Serialize};

        #[derive(Deserialize, Serialize)]
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
}

pub(crate) use param_struct;

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    let mut server = tide::new();

    let frontend_origin = if cfg!(debug_assertions) {
        "http://localhost:9000"
    } else {
        "https://stream-stash.com"
    };

    let cors_middleware = tide::security::CorsMiddleware::new()
        .allow_methods(
            "GET, POST, OPTIONS"
                .parse::<tide::http::headers::HeaderValue>()
                .unwrap(),
        )
        .allow_origin(tide::security::Origin::from(frontend_origin))
        .allow_credentials(true);

    server.with(cors_middleware);

    let cookie_domain = if cfg!(debug_assertions) {
        "" // TODO: Set logical cookie domain when testing locally
    } else {
        "stream-stash.com"
    };
    
    let session_middleware = tide::sessions::SessionMiddleware::new(
        tide::sessions::CookieStore::new(),
        std::env::var("TIDE_SECRET")
            .expect("Please provide a TIDE_SECRET envvar of at least 32 bytes")
            .as_bytes(),
    )
    // Disable secure cookies in development, some browsers don't support secure cookies on http://localhost
    .with_secure(!cfg!(debug_assertions))
    .with_same_site_policy(tide::http::cookies::SameSite::Strict)
    .with_cookie_domain(cookie_domain)
    // 2678400 seconds = 31 days
    .with_session_ttl(Some(std::time::Duration::from_secs(2678400)));

    server.with(session_middleware);

    server.at("/").get(|req: tide::Request<()>| async move {
        let username: Option<String> = req.session().get("username");

        match username {
            Some(username) => Ok(format!("You are logged in as '{}'", username)),
            None => Ok("You are not logged in".to_string()),
        }
    });

    server
        .at("/login/:username")
        .get(|mut req: tide::Request<()>| async move {
            let username = req.param("username").unwrap().to_owned();
            req.session_mut().insert("username", username).unwrap();

            Ok(tide::Redirect::new("/"))
        });

    server
        .at("/logout")
        .get(|mut req: tide::Request<()>| async move {
            req.session_mut().destroy();
            Ok(tide::Redirect::new("/"))
        });

    server.listen("localhost:8080").await?;

    Ok(())
}
