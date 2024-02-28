use tide::{Response, StatusCode};

async fn get_media(req: tide::Request<()>) -> tide::Result {
    let Some(username): Option<String> = req.session().get("username") else {
        return Ok(Response::builder(StatusCode::Forbidden)
            .header("X-Reason", "LoginRequired")
            .build());
    };

    let media_type = req.param("media_type").unwrap();
    let id = req.param("id").unwrap();

    Ok(Response::builder(StatusCode::Ok)
        .content_type("application/json")
        .body(
            match std::fs::read_to_string(format!("mediaDB/{username}/{media_type}/{id}.json")) {
                Ok(json) => json,
                _ => "default bomba".to_string(),
            },
        )
        .build())
}

pub fn new() -> tide::Server<()> {
    let mut server = tide::new();

    server.at("/:media_type/:id").get(get_media);

    server
}
