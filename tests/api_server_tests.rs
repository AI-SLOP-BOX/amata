//! `amata serve` used to be a stub that only printed an endpoint banner, so
//! nothing here could be exercised. These tests drive the routes directly and
//! then over a real loopback socket (request parsing, Content-Length handling,
//! response framing) to keep the server honest.
#![allow(clippy::field_reassign_with_default)]

use irasu_illustrator::cli::handlers::server::{route, serve, ApiResponse, ApiState};
use irasu_illustrator::core::document::{Document, Object, ObjectType};
use irasu_illustrator::core::path::FillStyle;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn response_json(response: &ApiResponse) -> serde_json::Value {
    serde_json::from_slice(&response.body).expect("response body must be JSON")
}

fn doc_with_rect() -> Document {
    let mut doc = Document::default();
    doc.width = 400.0;
    doc.height = 300.0;
    let mut rect = Object::new_rect("Base", 10.0, 20.0, 100.0, 50.0, 0.0);
    rect.fill = Some(FillStyle::solid([1.0, 0.0, 0.0, 1.0]));
    doc.add_object(rect);
    doc
}

#[test]
fn test_health_and_index_routes() {
    let mut api = ApiState::new(doc_with_rect());

    let health = route(&mut api, "GET", "/api/health", &[]);
    assert_eq!(health.status, 200);
    let value = response_json(&health);
    assert_eq!(value["status"], "ok");
    assert_eq!(value["requests"], 1);

    let index = route(&mut api, "GET", "/", &[]);
    assert_eq!(index.status, 200);
    let index_text = String::from_utf8(index.body).unwrap();
    assert!(index_text.contains("/api/document"));
    assert!(index_text.contains("/api/objects/rect"));
}

#[test]
fn test_document_and_objects_routes() {
    let mut api = ApiState::new(doc_with_rect());

    let document = response_json(&route(&mut api, "GET", "/api/document", &[]));
    assert_eq!(document["name"], "Untitled");
    assert_eq!(document["width"], 400.0);
    assert_eq!(document["object_count"], 1);
    assert_eq!(document["layers"][0]["object_count"], 1);

    // Trailing slash must resolve to the same route.
    let objects = response_json(&route(&mut api, "GET", "/api/objects/", &[]));
    assert_eq!(objects["count"], 1);
    assert_eq!(objects["objects"][0]["name"], "Base");
    assert_eq!(objects["objects"][0]["type"], "rect");
    assert_eq!(objects["objects"][0]["bbox"]["min_x"], 10.0);
    assert_eq!(objects["objects"][0]["bbox"]["max_x"], 110.0);
}

#[test]
fn test_create_shape_routes() {
    let mut api = ApiState::new(Document::default());

    let rect = route(
        &mut api,
        "POST",
        "/api/objects/rect",
        r##"{"name":"Card","x":5,"y":6,"width":50,"height":30,"fill":"#00ff00"}"##
            .as_bytes(),
    );
    assert_eq!(rect.status, 200);
    assert_eq!(response_json(&rect)["object_count"], 1);

    // `w`/`h` aliases and numeric fill channels.
    let ellipse = route(
        &mut api,
        "POST",
        "/api/objects/ellipse",
        br#"{"cx":100,"cy":100,"rx":20,"ry":10,"fill":[0,0,1,1]}"#,
    );
    assert_eq!(ellipse.status, 200, "{}", String::from_utf8_lossy(&ellipse.body));

    let path = route(
        &mut api,
        "POST",
        "/api/objects/path",
        br#"{"points":[[0,0],[10,0],[10,10]],"closed":true}"#,
    );
    assert_eq!(path.status, 200);

    let objects = response_json(&route(&mut api, "GET", "/api/objects", &[]));
    assert_eq!(objects["count"], 3);
    assert_eq!(objects["objects"][0]["type"], "rect");
    assert_eq!(objects["objects"][1]["type"], "ellipse");
    assert_eq!(objects["objects"][2]["type"], "path");

    let types: Vec<&str> = api
        .document
        .all_objects()
        .map(|(_, o)| match &o.object_type {
            ObjectType::Rectangle { .. } => "rect",
            ObjectType::Ellipse { .. } => "ellipse",
            ObjectType::Path(_) => "path",
            _ => "other",
        })
        .collect();
    assert_eq!(types, vec!["rect", "ellipse", "path"]);
}

#[test]
fn test_error_responses() {
    let mut api = ApiState::new(Document::default());

    let missing = route(&mut api, "GET", "/api/nope", &[]);
    assert_eq!(missing.status, 404);

    let wrong_method = route(&mut api, "DELETE", "/api/document", &[]);
    assert_eq!(wrong_method.status, 405);

    let bad_json = route(&mut api, "POST", "/api/objects/rect", b"{not json");
    assert_eq!(bad_json.status, 400);

    let zero_size = route(
        &mut api,
        "POST",
        "/api/objects/rect",
        br#"{"width":0,"height":10}"#,
    );
    assert_eq!(zero_size.status, 422);

    let nan_size = route(
        &mut api,
        "POST",
        "/api/objects/ellipse",
        br#"{"rx":"NaN","ry":10}"#,
    );
    assert_eq!(nan_size.status, 422);

    let bad_points = route(
        &mut api,
        "POST",
        "/api/objects/path",
        br#"{"points":[[0,0],[1,"x"]]}"#,
    );
    assert_eq!(bad_points.status, 422);

    // Rejected requests must not have mutated the document.
    assert_eq!(api.document.all_objects().count(), 0);
}

#[test]
fn test_script_route_accepts_json_and_raw_bodies() {
    let mut api = ApiState::new(Document::default());

    let script = r#"
        let objs = [];
        objs.push(#{ type: "rect", name: "From Script", x: 0.0, y: 0.0, w: 25.0, h: 25.0 });
        #{ width: 100.0, height: 100.0, objects: objs }
    "#;
    let json_body = serde_json::json!({ "script": script }).to_string();
    let response = route(&mut api, "POST", "/api/script", json_body.as_bytes());
    assert_eq!(response.status, 200, "{}", String::from_utf8_lossy(&response.body));
    let value = response_json(&response);
    assert_eq!(value["objects_before"], 0);
    assert_eq!(value["objects_after"], 1);
    assert_eq!(api.document.all_objects().count(), 1);

    // Raw Rhai body (no JSON wrapper).
    let raw = route(&mut api, "POST", "/api/script", script.as_bytes());
    assert_eq!(raw.status, 200);
    assert_eq!(api.document.all_objects().count(), 2);

    let broken = route(&mut api, "POST", "/api/script", b"let x = ;");
    assert_eq!(broken.status, 422);

    let empty = route(&mut api, "POST", "/api/script", &[]);
    assert_eq!(empty.status, 422);
}

#[test]
fn test_export_svg_route_inline_and_to_disk() {
    let mut api = ApiState::new(doc_with_rect());

    let inline = route(&mut api, "POST", "/api/export/svg", &[]);
    assert_eq!(inline.status, 200);
    assert_eq!(inline.content_type, "image/svg+xml; charset=utf-8");
    let svg = String::from_utf8(inline.body).unwrap();
    assert!(svg.starts_with("<?xml"));
    assert!(svg.contains("</svg>"));

    let dir = std::env::temp_dir().join("amata_api_export");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let target = dir.join("out.svg");
    let target_uri = target.display().to_string().replace('/', "%2F");
    let response = route(
        &mut api,
        "POST",
        &format!("/api/export/svg?path={target_uri}"),
        &[],
    );
    assert_eq!(response.status, 200, "{}", String::from_utf8_lossy(&response.body));
    assert!(target.exists(), "export must write the requested file");
    let written = std::fs::read_to_string(&target).unwrap();
    assert!(written.contains("</svg>"));
    // Atomic writer must not leave a `.tmp` sibling behind.
    assert!(!dir.join("out.svg.tmp").exists());

    let _ = std::fs::remove_dir_all(&dir);
}


fn start_server(api: ApiState) -> (SocketAddr, Arc<AtomicBool>, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_thread = stop.clone();
    let thread = std::thread::spawn(move || {
        serve(listener, api, &stop_for_thread).expect("server loop");
    });
    (addr, stop, thread)
}

fn send_raw(addr: SocketAddr, request: &str) -> String {
    let mut stream = TcpStream::connect(addr).expect("connect to test server");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    stream.flush().unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

fn http_post(addr: SocketAddr, path: &str, body: &str) -> String {
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\n\r\n{body}",
        body.len()
    );
    send_raw(addr, &request)
}

#[test]
fn test_http_roundtrip_get_and_post() {
    let (addr, stop, thread) = start_server(ApiState::new(doc_with_rect()));

    let health = send_raw(
        addr,
        "GET /api/health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    );
    assert!(health.starts_with("HTTP/1.1 200 OK"), "got: {health}");
    assert!(health.contains("Content-Type: application/json"));
    assert!(health.contains("\"status\": \"ok\""));

    let created = http_post(addr, "/api/objects/rect", r#"{"name":"Via HTTP","width":10,"height":10}"#);
    assert!(created.starts_with("HTTP/1.1 200 OK"), "got: {created}");
    assert!(created.contains("\"object_count\": 2"));

    // The server keeps state between connections.
    let objects = send_raw(addr, "GET /api/objects HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
    assert!(objects.contains("\"count\": 2"));
    assert!(objects.contains("Via HTTP"));

    // Unknown route still answers with a framed, parseable response.
    let missing = send_raw(addr, "GET /api/ghost HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
    assert!(missing.starts_with("HTTP/1.1 404 Not Found"), "got: {missing}");

    stop.store(true, Ordering::Relaxed);
    thread.join().expect("server thread must stop cleanly");
}

#[test]
fn test_http_rejects_malformed_and_chunked_requests() {
    let (addr, stop, thread) = start_server(ApiState::default());

    let chunked = send_raw(
        addr,
        "POST /api/objects/rect HTTP/1.1\r\nHost: 127.0.0.1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
    );
    assert!(chunked.starts_with("HTTP/1.1 400 Bad Request"), "got: {chunked}");

    let garbage = send_raw(addr, "\r\n\r\n");
    assert!(garbage.contains("400"), "got: {garbage}");

    stop.store(true, Ordering::Relaxed);
    thread.join().expect("server thread must stop cleanly");
}

