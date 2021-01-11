use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::convert::Infallible;
use std::io::Read;
use std::net::SocketAddr;

mod storage {
    use std::fs::File;
    use std::io::Error;
    use std::path::Path;

    #[derive(Copy, Clone)]
    pub struct Storage {}

    impl Storage {
        pub fn new() -> Self {
            Storage {}
        }
        pub fn get(self, filename: &str) -> Result<File, Error> {
            let path = Path::new(filename);
            File::open(path)
        }
    }
}

#[derive(Copy, Clone)]
struct HTTPServer {
    storage: storage::Storage,
}

impl HTTPServer {
    fn new(storage: storage::Storage) -> Self {
        HTTPServer { storage }
    }
}

impl HTTPServer {
    async fn handler(self, req: Request<Body>) -> Result<Response<Body>, Infallible> {
        match req.method() {
            &Method::GET => {
                let mut f = self.storage.get(req.uri().path()).unwrap();
                let mut x = String::new();
                f.read_to_string(&mut x).unwrap();
                Ok(Response::new(x.into()))
            }
            _ => Ok(Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body("Method Not Allowed".into())
                .unwrap()),
        }
    }
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    let s = storage::Storage::new();

    let http = HTTPServer::new(s);

    let svc =
        make_service_fn(
            |_| async move { Ok::<_, Infallible>(service_fn(move |r| http.handler(r))) },
        );

    let server = Server::bind(&addr).serve(svc);

    println!("Listening on 0.0.0.0:3000 ...");
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
