mod test_router;

use tide::prelude::*;

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

    server.at("/").get(|_| async move { Ok("/ route (root)") });
    server.at("/test").nest(test_router::new());

    server.listen("localhost:8080").await?;

    Ok(())
}
