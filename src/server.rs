
use crate::deps;
use crate::tcp;

use deps::tokio;
use deps::hyper;
use deps::hyper_util;
use deps::log;
use deps::http_body_util;
use deps::serde_json;
use deps::futures;
use deps::http_body;

use hyper::body::Bytes;
use hyper::body::Frame;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper::{Method, StatusCode};
use hyper_util::rt::TokioIo;
use http_body_util::{combinators::BoxBody, BodyExt, Full, Empty};
use futures::{Stream, StreamExt};

use deps::parking_lot::RwLock;
use std::sync::Arc;
use std::convert::Infallible;
use std::fmt::{Display, Formatter};

use std::net::TcpListener;

static ZEROS: [u8; 65536] = [0u8; 65536];

static INDEX_HTML: &'static str = include_str!("../static/index.html");

#[derive(Debug, Clone, Copy)]
pub enum HttpVersion {
    Http1,
    Http2,
    Http3,
}

impl Display for HttpVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpVersion::Http1 => write!(f, "HTTP/1.1"),
            HttpVersion::Http2 => write!(f, "HTTP/2"),
            HttpVersion::Http3 => write!(f, "HTTP/3"),
        }
    }
}

#[allow(dead_code)]
fn empty() -> BoxBody<Bytes, Infallible> {
    Empty::<Bytes>::new()
        .boxed()
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, Infallible> {
    Full::new(chunk.into())
        .boxed()
}

fn json_response(status: StatusCode, http_version: HttpVersion, v: serde_json::Value) -> Response<BoxBody<Bytes, Infallible>> {
    let body = serde_json::to_vec(&v).unwrap();
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("X-Http-Version", http_version.to_string())
        .body(full(body))
        .unwrap()
}

async fn handle_request(req: Request<hyper::body::Incoming>, http_version: HttpVersion) -> Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
    let res = match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            let mut res = Response::new(full(INDEX_HTML));
            res.headers_mut().insert("Content-Type", "text/html".parse().unwrap());
            res.headers_mut().insert("X-Http-Version", http_version.to_string().parse().unwrap());
            res
        }
        (&Method::POST, "/upload") => {
            let mut body = req.into_body();
            let mut bytes: usize = 0;
            while let Some(frame) = body.frame().await {
                match frame {
                    Ok(frame) => {
                        if let Ok(data) = frame.into_data() {
                            bytes += data.len();
                        };
                    }
                    Err(e) => {
                        log::warn!("frame error: {:?}", e);
                        return Ok(json_response(StatusCode::BAD_REQUEST, http_version, serde_json::json!({
                            "error": "frame error"
                        })));
                    }
                }
            }

            json_response(StatusCode::OK, http_version, serde_json::json!({
                "uploaded_bytes": bytes
            }))
        }
        (&Method::GET, uri) => {
            let download_prefix = "/download/";
            if uri.starts_with(download_prefix) {
                let len = &uri[download_prefix.len()..];
                let len = len.parse::<usize>().unwrap_or(0);
                if len > 1073741824 {
                    json_response(StatusCode::BAD_REQUEST, http_version, serde_json::json!({
                        "error": "too large"
                    }))
                } else {
                    let mut remaining = len;
                    let body = futures::stream::repeat_with(move || {
                        let chunk = std::cmp::min(remaining, 65536);
                        remaining -= chunk;
                        if chunk > 0 {
                            Some(Bytes::from(&ZEROS[..chunk]))
                        } else {
                            None
                        }
                    }).take_while(|x| futures::future::ready(x.is_some())).map(|x| Ok(Frame::data(x.unwrap())));
                    let mut res = Response::new(BoxBody::new(http_body_util::StreamBody::new(body)));
                    res.headers_mut().insert("Content-Type", "application/octet-stream".parse().unwrap());
                    res.headers_mut().insert("Content-Length", len.to_string().parse().unwrap());
                    res.headers_mut().insert("Cache-Control", "no-store".parse().unwrap());
                    res.headers_mut().insert("X-Http-Version", http_version.to_string().parse().unwrap());
                    *res.status_mut() = StatusCode::OK;
                    res
                }
            } else {
                json_response(StatusCode::NOT_FOUND, http_version, serde_json::json!({
                    "error": "not found"
                }))
            }
        }
        _ => {
            json_response(StatusCode::METHOD_NOT_ALLOWED, http_version, serde_json::json!({
                "error": "method not allowed"
            }))
        }
    };
    Ok(res)
}

pub struct PlainHttpServer {
    listener: TcpListener,
}

impl PlainHttpServer {
    pub fn new_from_listener(listener: TcpListener) -> Self {
        Self { listener }
    }

    pub fn new(port: u16, bind_device: Option<&[u8]>) -> Result<Self, std::io::Error> {
        let listener = tcp::listen(port, None, bind_device)?;
        Ok(Self { listener })
    }

    async fn run(&self) {
        let listener = tokio::net::TcpListener::from_std(self.listener.try_clone().unwrap()).unwrap();
        loop {
            let stream = if let Ok((stream, _)) = listener.accept().await {
                stream
            } else {
                continue;
            };

            let io = TokioIo::new(stream);
            tokio::task::spawn(async move {
                let service = service_fn(|req: _| {
                    let http_version = HttpVersion::Http1;
                    handle_request(req, http_version)
                });
                let conn = http1::Builder::new().serve_connection(io, service);
                if let Err(e) = conn.await {
                    log::error!("http1 connection error: {:?}", e);
                }
            });
        }
    }

    pub fn start(self) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(self.run());
        })
    }
}

