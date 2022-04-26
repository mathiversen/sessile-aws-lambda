// #![forbid(unsafe_code)]
// #![deny(
//     clippy::dbg_macro,
//     missing_copy_implementations,
//     rustdoc::missing_crate_level_docs,
//     missing_debug_implementations,
//     missing_docs,
//     nonstandard_style,
//     unused_qualifications
// )]
use std::sync::Arc;

use lambda_http::{service_fn, tower::BoxError, Body as LambdaBody, Request, RequestExt, Response};
use std::str::FromStr;
use trillium::{Conn, Handler, Headers};
use trillium_http::{Conn as HttpConn, Method, Synthetic};

mod context;
pub use context::LambdaConnExt;
use context::LambdaContext;

pub fn run(handler: impl Handler) {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run_async(handler));
}

pub async fn run_async(mut handler: impl Handler) {
    let mut info = "aws lambda".into();
    handler.init(&mut info).await;

    let handler = Arc::new(handler);

    lambda_http::run(service_fn(move |req: Request| {
        let ctx = req.lambda_context();
        let mut conn = lambda_req_to_conn(req);
        conn.state_mut().insert(LambdaContext::new(ctx));

        let handler_clone = handler.clone();

        async move {
            let conn = run_handler(conn, handler_clone).await;
            conn_to_res(conn).await
        }
    }))
    .await
    .unwrap();
}

async fn run_handler(conn: HttpConn<Synthetic>, handler: Arc<impl Handler>) -> Conn {
    let conn = handler.run(conn.into()).await;
    handler.before_send(conn).await
}

fn lambda_req_to_conn(req: Request) -> HttpConn<Synthetic> {
    let (parts, lambda_body) = req.into_parts();

    let method = Method::from_str(&parts.method.to_string()).unwrap();
    let path = parts.uri.path();

    let mut conn = match lambda_body {
        LambdaBody::Empty => HttpConn::new_synthetic(method, path, None),
        LambdaBody::Text(data) => HttpConn::new_synthetic(method, path, data),
        LambdaBody::Binary(data) => HttpConn::new_synthetic(method, path, data),
    };

    let mut headers = Headers::new();

    for (name, value) in parts.headers {
        if let Some(name) = name {
            headers.append(name.as_str().to_string(), value.as_bytes().to_owned());
        } else {
            headers.append(value.to_str().unwrap().to_string(), "")
        }
    }

    conn.request_headers_mut().extend(headers);

    conn
}

async fn conn_to_res(mut conn: Conn) -> Result<Response<String>, BoxError> {
    Ok(Response::new(conn.request_body_string().await.unwrap()))
}
