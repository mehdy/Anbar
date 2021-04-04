use hyper::service::{make_service_fn, service_fn};
use hyper::Server;
use std::clone::Clone;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

use anbar::drivers::web_server::App;
use anbar::interactors::storage::Storage;

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    let mut s = Storage::new("/home/mehdy/tmp/anbar");
    s.new_user("mehdy", "Mehdy", "ABC1234", "AbC1Zxv");
    s.create_bucket("mehdy", "buck");

    let storage = Arc::new(Mutex::new(s));

    let service = make_service_fn(move |_conn| {
        let app = App {
            storage: storage.clone(),
        };
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let app = app.clone();
                app.handle(req)
            }))
        }
    });

    let server = Server::bind(&addr).serve(service);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
