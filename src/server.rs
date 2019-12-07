/// Server is an HTTP(S) server for answering Kubelet callbacks.
///
/// Logs and exec calls are the main things that a server should handle.
use futures::future;
use hyper::rt::Future;
use hyper::service::service_fn_ok;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use kube::client::APIClient;
use log::{error, info};
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;

use crate::kubelet::Provider;
use crate::pod::KubePod;

/// Start the Krustlet HTTP(S) server
///
/// FIXME: The env var read should be moved up to main.rs
pub fn start_webserver<T: Provider + Clone + Send + Sync>(
    provider: T,
    address: &SocketAddr,
) -> Result<(), failure::Error> {
    //let shared_provider = Arc::new(Mutex::new(provider.clone()));
    let provider = provider.clone();
    let svc = || {
        //let prov = shared_provider.clone();
        let shared_provider = Arc::new(Mutex::new(provider.clone()));
        service_fn_ok( |req| {
            let path: Vec<&str> = req.uri().path().split('/').collect();
            let path_len = path.len();
            if path_len < 2 {
                return get_ping();
            }
            match (req.method(), path[1], path_len) {
                (&Method::GET, "containerLogs", 5) => {
                    let p = shared_provider.lock().unwrap();
                    get_container_logs(p.clone(), req)
                }
                //(&Method::POST, "exec", 5) => post_exec(prov.clone(), req),
                _ => {
                    let mut response = Response::new(Body::from("Not Found"));
                    *response.status_mut() = StatusCode::NOT_FOUND;
                    response
                }
            }
        })
    };
    let server = Server::bind(address)
        .serve(svc)
        .map_err(|e| error!("HTTP server error: {}", e));

    info!("starting webserver at: {:?}", address;
    hyper::rt::run(server);
    Ok(())
}

/// Convenience type for hyper
type BoxFut = Box<dyn futures::future::Future<Item = Response<Body>, Error = hyper::Error> + Send>;

struct WebServer<P: Provider> {
    provider: P,
    client: APIClient,
}

/// Return a simple status message
fn get_ping() -> Response<Body> {
    Response::new(Body::from("this is the Krustlet HTTP server"))
}

/// Get the logs from the running WASM module
///
/// Implements the kubelet path /containerLogs/{namespace}/{pod}/{container}
fn get_container_logs<T: Provider>(provider: T, _req: Request<Body>) -> Response<Body> {
    // TODO: extract the right data from the request.
    let pod = KubePod {
        metadata: Default::default(),
        spec: Default::default(),
        status: Default::default(),
        types: Default::default(),
    };
    match provider.logs(pod) {
        Ok(lines) => Response::new(Body::from(lines.join("\n"))),
        // TODO: THis should detect not implemented vs. regular error
        Err(e) => {
            error!("Error fetching logs: {}", e);
            let mut res = Response::new(Body::from("Not Implemented"));
            *res.status_mut() = StatusCode::NOT_IMPLEMENTED;
            res
        }
    }
}
/// Run a pod exec command and get the output
///
/// Implements the kubelet path /exec/{namespace}/{pod}/{container}
fn post_exec<T: Provider>(provider: T, _req: Request<Body>) -> Response<Body> {
    let mut res = Response::new(Body::from("Not Implemented"));
    *res.status_mut() = StatusCode::NOT_IMPLEMENTED;
    res
}