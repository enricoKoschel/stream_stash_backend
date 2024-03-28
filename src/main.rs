mod v1router;

macro_rules! serde_struct {
    ($struct_name:ident, $($field_name:ident: $field_type:ty = $field_default:expr),+) => {
        #[derive(serde::Deserialize, serde::Serialize, Debug)]
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
    ($struct_name:ident, $($field_name:ident: $field_type:ty),+) => {
        #[derive(serde::Deserialize, serde::Serialize, Debug)]
        struct $struct_name {
            $(
                $field_name: $field_type,
            )*
        }
    };
}

pub(crate) use serde_struct;

fn setup_cors(server: &mut tide::Server<()>) {
    let frontend_url = if cfg!(debug_assertions) {
        "http://localhost:9000"
    } else {
        "https://stream-stash.com"
    };

    let allowed_methods: tide::http::headers::HeaderValue =
        "GET, POST, DELETE, OPTIONS".parse().unwrap();

    let allowed_headers: tide::http::headers::HeaderValue = "content-type".parse().unwrap();

    let cors_middleware = tide::security::CorsMiddleware::new()
        .allow_methods(allowed_methods)
        .allow_headers(allowed_headers)
        .allow_origin(tide::security::Origin::from(frontend_url))
        .allow_credentials(true);

    server.with(cors_middleware);
}

fn setup_sessions(server: &mut tide::Server<()>) {
    let cookie_domain = if cfg!(debug_assertions) {
        "localhost"
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
}

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    let mut server = tide::new();

    setup_cors(&mut server);
    setup_sessions(&mut server);

    server.at("/v1").nest(v1router::new());

    server.listen("localhost:8080").await?;

    Ok(())
}
