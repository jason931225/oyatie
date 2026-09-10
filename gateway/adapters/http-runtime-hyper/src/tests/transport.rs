use super::*;

#[tokio::test]
async fn boundary_conversion_round_trip_identity() {
    let original: Vec<u8> = (0u8..=255).collect();
    let resp = HttpResponse::new(200).with_body(original.clone());
    let hyper_resp = to_hyper_response(resp);
    let (parts, body) = hyper_resp.into_parts();
    assert_eq!(parts.status.as_u16(), 200);
    let drained = body
        .collect()
        .await
        .expect("Full<Bytes> never errors on collect")
        .to_bytes();
    assert_eq!(
        drained.as_ref(),
        original.as_slice(),
        "Vec<u8> -> Bytes -> Full -> collect -> Bytes must be byte-identical"
    );
    assert_eq!(drained.len(), 256);
    assert_eq!(drained[0], 0);
    assert_eq!(drained[255], 255);
}

#[tokio::test]
async fn boundary_conversion_empty_body_round_trip() {
    let resp = HttpResponse::new(204).with_body(Vec::new());
    let hyper_resp = to_hyper_response(resp);
    let (parts, body) = hyper_resp.into_parts();
    assert_eq!(parts.status.as_u16(), 204);
    let drained = body.collect().await.unwrap().to_bytes();
    assert!(drained.is_empty());
}

#[tokio::test]
async fn serve_one_connection_serves_loopback_request() {
    use std::io::{Read, Write};

    let mut router: Router<SyncHandler> = Router::new();
    router
        .route(HttpMethod::Get, "/healthz", ok_handler(b"ok"))
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local listener");
    let addr = listener.local_addr().expect("listener has local addr");
    let server = tokio::spawn(async move {
        serve_one_connection(
            listener,
            Arc::new(router),
            Arc::new(empty_chain()),
            ServerConfig::default().with_max_body_bytes(0),
        )
        .await
    });

    let response = tokio::task::spawn_blocking(move || {
        let mut stream = std::net::TcpStream::connect(addr).expect("connect loopback");
        stream
            .write_all(b"GET /healthz HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
            .expect("write request");
        let mut response = String::new();
        stream.read_to_string(&mut response).expect("read response");
        response
    })
    .await
    .expect("client task joins");

    server
        .await
        .expect("server task joins")
        .expect("serves one connection");
    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert!(response.ends_with("ok"));
}

#[tokio::test]
async fn serve_n_connections_serves_bounded_loopback_requests() {
    use std::io::{Read, Write};

    let mut router: Router<SyncHandler> = Router::new();
    router
        .route(HttpMethod::Get, "/healthz", ok_handler(b"health"))
        .unwrap();
    router
        .route(HttpMethod::Get, "/livez", ok_handler(b"live"))
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind local listener");
    let addr = listener.local_addr().expect("listener has local addr");
    let server = tokio::spawn(async move {
        serve_n_connections(
            listener,
            Arc::new(router),
            Arc::new(empty_chain()),
            ServerConfig::default().with_max_body_bytes(0),
            2,
        )
        .await
    });

    let responses = tokio::task::spawn_blocking(move || {
        ["/healthz", "/livez"].map(|path| {
            let mut stream = std::net::TcpStream::connect(addr).expect("connect loopback");
            stream
                .write_all(
                    format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
                        .as_bytes(),
                )
                .expect("write request");
            let mut response = String::new();
            stream.read_to_string(&mut response).expect("read response");
            response
        })
    })
    .await
    .expect("client task joins");

    server
        .await
        .expect("server task joins")
        .expect("serves bounded connections");
    assert!(responses[0].starts_with("HTTP/1.1 200 OK"));
    assert!(responses[0].ends_with("health"));
    assert!(responses[1].starts_with("HTTP/1.1 200 OK"));
    assert!(responses[1].ends_with("live"));
}
