use lambda_http::{http::StatusCode, tower::BoxError, Body, Request, Response};
use std::str::FromStr;
use trillium::{Conn, Headers, Status};
use trillium_http::{transport::BoxedTransport, Conn as HttpConn, Method, Synthetic};

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
pub async fn conn_to_res(conn: Conn) -> Result<Response<Body>, BoxError> {
    let mut conn = conn.into_inner();
    let status = conn.status().unwrap_or(Status::NotFound);
    let (body, is_base64_encoded) = response_body(&mut conn).await;

    log::debug!("{:?}", &body);

    let mut response = match (body, is_base64_encoded) {
        (Some(body), _) => Response::new(Body::Text(body)),
        (None, _) => Response::new(Body::Empty),
    };

    *response.status_mut() = StatusCode::try_from(status as u16)?;

    log::trace!("{:?}", &response);

    Ok(response)
}

async fn response_body(conn: &mut HttpConn<BoxedTransport>) -> (Option<String>, bool) {
    match conn.take_response_body() {
        Some(body) => {
            let bytes = body.into_bytes().await.unwrap();
            match String::from_utf8(bytes.to_vec()) {
                Ok(string) => (Some(string), false),
                Err(e) => (Some(base64::encode(e.into_bytes())), true),
            }
        }
        None => (None, false),
    }
}
