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

use lambda_http::{service_fn, Request, RequestExt};
use trillium::{Conn, Handler};
use trillium_http::{Conn as HttpConn, Synthetic};

mod context;
pub use context::LambdaConnExt;
use context::LambdaContext;

mod utils;

async fn run_handler(conn: HttpConn<Synthetic>, handler: Arc<impl Handler>) -> Conn {
    let conn = handler.run(conn.into()).await;
    handler.before_send(conn).await
}

pub async fn run_async(mut handler: impl Handler) {
    let mut info = "aws lambda".into();
    handler.init(&mut info).await;

    let handler = Arc::new(handler);

    lambda_http::run(service_fn(move |req: Request| {
        log::debug!("1 {:?}", req);
        let ctx = req.lambda_context();
        log::debug!("2 {:?}", ctx);
        let mut conn = utils::lambda_req_to_conn(req);
        log::debug!("3 {:?}", conn);
        conn.state_mut().insert(LambdaContext::new(ctx));

        let handler_clone = handler.clone();

        async move {
            let conn = run_handler(conn, handler_clone).await;
            log::debug!("4 {:?}", conn);
            let res = utils::conn_to_res(conn).await.unwrap();
            log::debug!("5 {:?}", res);
            Ok(res)
        }
    }))
    .await
    .unwrap();
}

pub fn run(handler: impl Handler) {
    log::debug!("start");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run_async(handler));
}
