mod db_router;

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

    /*let cors = tide::security::CorsMiddleware::new()
        .allow_methods(
            "GET, POST, OPTIONS"
                .parse::<tide::http::headers::HeaderValue>()
                .unwrap(),
        )
        .allow_origin(tide::security::Origin::from("*"))
        .allow_credentials(false);

    server.with(cors);*/

    server.with(tide::sessions::SessionMiddleware::new(
        tide::sessions::MemoryStore::new(),
        std::env::var("TIDE_SECRET")
            .expect(
                "Please provide a TIDE_SECRET value of at least 32 bytes in order to run this example",
            )
            .as_bytes(),
    ));

    server.at("/").get(|req: tide::Request<()>| async move {
        let username: Option<String> = req.session().get("username");
        println!(
            "{:?} {:?} {:?}",
            req.host(),
            req.local_addr(),
            req.peer_addr()
        );

        match username {
            Some(username) => Ok(format!("you are logged in as '{}'", username)),
            None => Ok("you are not logged in".to_string()),
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

    server.at("/db").nest(db_router::new());

    // TODO: In production have different routers for api.stream-stash.com and stream-stash.com
    server
        .listen(vec!["localhost:8080", "127.0.0.1:8080"])
        .await?;

    Ok(())
}
