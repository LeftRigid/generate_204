use std::convert::Infallible;
use std::net::SocketAddr;

use http_body_util::Empty;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 80));

    let listener = TcpListener::bind(addr).await?;

    // start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;

        // sse an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            // bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, service_fn(generate_204))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

async fn generate_204(
    _: Request<hyper::body::Incoming>,
) -> Result<Response<Empty<Bytes>>, Infallible> {
    Ok(Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Empty::new())
        .unwrap())
}
