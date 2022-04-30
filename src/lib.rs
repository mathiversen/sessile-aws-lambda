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
use std::{pin::Pin, sync::Arc};

use futures_lite::Future;
use lambda_http::{Body, Context, Request, RequestExt, Response, Service};
use trillium::{Conn, Handler};
use trillium_http::{Conn as HttpConn, Synthetic};

mod context;
pub use context::LambdaConnExt;
use context::LambdaContext;

mod utils;

// pub async fn run_async(mut handler: impl Handler) {
//     let mut info = "aws lambda".into();
//     handler.init(&mut info).await;

//     let handler = Arc::new(handler);

//     lambda_http::run(service_fn(move |req: Request| {
//         log::debug!("1 {:?}", req);
//         let ctx = req.lambda_context();
//         log::debug!("2 {:?}", ctx);
//         let mut conn = utils::lambda_req_to_conn(req);
//         log::debug!("3 {:?}", conn);
//         conn.state_mut().insert(LambdaContext::new(ctx));

//         let handler_clone = handler.clone();

//         async move {
//             let conn = run_handler(conn, handler_clone).await;
//             log::debug!("4 {:?}", conn);
//             let res = utils::conn_to_res(conn).await.unwrap();
//             log::debug!("5 {:?}", res);
//             Ok(res)
//         }
//     }))
//     .await
//     .unwrap();
// }

#[derive(Debug)]
pub struct HandlerWrapper<H>(Arc<H>);

impl<H: Handler> Service<Request> for HandlerWrapper<H> {
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;
    type Response = Response<Body>;

    fn call(&mut self, req: Request) -> Self::Future {
        let ctx = req.lambda_context();
        log::trace!("{:?}", &ctx);
        Box::pin(handler_fn(req, ctx, Arc::clone(&self.0)))
    }

    #[allow(unconditional_recursion)]
    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.poll_ready(cx).map_err(Into::into)
    }
}

async fn run_handler(conn: HttpConn<Synthetic>, handler: Arc<impl Handler>) -> Conn {
    let conn = handler.run(conn.into()).await;
    handler.before_send(conn).await
}

async fn handler_fn(
    req: Request,
    ctx: Context,
    handler: Arc<impl Handler>,
) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync>> {
    log::trace!("{:?}", req);
    let mut conn = utils::lambda_req_to_conn(req);
    log::trace!("{:?}", conn);
    conn.state_mut().insert(LambdaContext::new(ctx));
    let conn = run_handler(conn, handler).await;
    log::trace!("{:?}", conn);
    utils::conn_to_res(conn).await
}

pub async fn run_async(mut handler: impl Handler) {
    let mut info = "aws lambda".into();
    handler.init(&mut info).await;

    lambda_http::run(HandlerWrapper(Arc::new(handler)))
        .await
        .unwrap()
}

pub fn run(handler: impl Handler) {
    log::debug!("start");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run_async(handler));
}
