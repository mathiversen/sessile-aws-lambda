use lambda_http::{http::StatusCode, tower::BoxError, Body, Request, Response};
use std::str::FromStr;
use trillium::{Conn, Headers};
use trillium_http::{Conn as HttpConn, Method, Synthetic};

pub fn lambda_req_to_conn(req: Request) -> HttpConn<Synthetic> {
    let (parts, lambda_body) = req.into_parts();

    let method = Method::from_str(&parts.method.to_string()).unwrap();
    let path = parts.uri.path();

    let mut conn = match lambda_body {
        Body::Empty => HttpConn::new_synthetic(method, path, None),
        Body::Text(data) => HttpConn::new_synthetic(method, path, data),
        Body::Binary(data) => HttpConn::new_synthetic(method, path, data),
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

// TODO: Add everything else...
pub async fn conn_to_res(mut conn: Conn) -> Result<Response<Body>, BoxError> {
    let body = conn
        .request_body()
        .await
        .read_bytes()
        .await
        .expect("request body");

    let mut response = Response::new(Body::Binary(body));
    *response.status_mut() = StatusCode::try_from(conn.status().unwrap() as u16)?;

    log::trace!("{:?}", response);

    Ok(response)
}
