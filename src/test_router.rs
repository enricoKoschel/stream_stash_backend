use crate::param_struct;
use tide::{Body, Response, StatusCode};

param_struct!(Page, size: Option<i8> = None, offset: u8 = 10);

async fn page_param(req: tide::Request<()>) -> tide::Result {
    let page: Page = req.query()?;
    let mut res = Response::new(StatusCode::Ok);
    res.set_body(Body::from_json(&page)?);

    Ok(res)
}

pub fn new() -> tide::Server<()> {
    let mut server = tide::new();

    server
        .at("/")
        .get(|_| async move { Ok("/test/ route (root of /test)") });
    server.at("/page_param").get(page_param);

    server
}
