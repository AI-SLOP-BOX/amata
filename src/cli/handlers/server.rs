//! `amata serve` — local HTTP API for external tool integration.
//!
//! Before this module existed the handler only printed a help banner and exited
//! ("currently stub"), so none of the documented endpoints could actually be
//! called. The server is deliberately dependency-free: it speaks HTTP/1.1 over
//! `std::net::TcpListener` so the binary keeps shipping with no async runtime.
//!
//! * Binds loopback only (`127.0.0.1`) — it is a local integration surface, not
//!   a network service.
//! * Answers one request per connection (`Connection: close`).
//! * Header/body sizes are capped, and `Transfer-Encoding: chunked` is rejected
//!   rather than guessed at.
//!
//! Routes (all JSON unless noted):
//!
//! | Method | Path                   | Purpose                          |
//! |--------|------------------------|----------------------------------|
//! | GET    | `/`                    | Plain-text route index           |
//! | GET    | `/api/health`          | Liveness probe                   |
//! | GET    | `/api/document`        | Document + layer summary         |
//! | GET    | `/api/objects`         | Every object with transform/bbox |
//! | POST   | `/api/objects/rect`    | Create a rectangle               |
//! | POST   | `/api/objects/ellipse` | Create an ellipse                |
//! | POST   | `/api/objects/path`    | Create a polyline/polygon        |
//! | POST   | `/api/script`          | Run a Rhai script                |
//! | POST   | `/api/export/svg`      | Export SVG (`?path=` = write)    |

use super::common::load_any_document;
use crate::core::document::{Document, Object, ObjectType, Transform};
use crate::core::path::{AnchorPoint, FillStyle, PathData};
use crate::core::state::AppState;
use crate::io::svg::parse_svg_color;
use crate::plugin::script::ScriptEngine;
use serde_json::{json, Value};
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Cap on the request line + headers.
const MAX_HEADER_BYTES: usize = 16 * 1024;
/// Cap on a request body (scripts and paths only, so this is generous).
const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;
/// Per-connection read/write timeout.
const IO_TIMEOUT: Duration = Duration::from_secs(10);

/// Mutable state shared by every request served by one `amata serve` process.
///
/// Note: `AppState` is deliberately *not* part of this struct — it owns
/// `Box<dyn Command>` history entries which are not `Send`, and the Rhai
/// script runner does not read it. Keeping `ApiState` `Send` lets the server be
/// driven from a worker thread (which is also what the tests do).
pub struct ApiState {
    pub document: Document,
    pub request_count: u64,
}

impl ApiState {
    pub fn new(document: Document) -> Self {
        Self {
            document,
            request_count: 0,
        }
    }
}

impl Default for ApiState {
    fn default() -> Self {
        Self::new(Document::default())
    }
}

/// A fully rendered HTTP response.
pub struct ApiResponse {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
}

impl ApiResponse {
    pub fn json(status: u16, value: &Value) -> Self {
        Self {
            status,
            content_type: "application/json; charset=utf-8".to_string(),
            body: serde_json::to_vec_pretty(value).unwrap_or_else(|_| b"{}".to_vec()),
        }
    }

    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type: "text/plain; charset=utf-8".to_string(),
            body: body.into().into_bytes(),
        }
    }

    pub fn bytes(status: u16, content_type: &str, body: Vec<u8>) -> Self {
        Self {
            status,
            content_type: content_type.to_string(),
            body,
        }
    }

    pub fn error(status: u16, message: impl Into<String>) -> Self {
        Self::json(status, &json!({ "error": message.into() }))
    }

    fn status_text(&self) -> &'static str {
        match self.status {
            200 => "OK",
            400 => "Bad Request",
            404 => "Not Found",
            405 => "Method Not Allowed",
            411 => "Length Required",
            413 => "Payload Too Large",
            415 => "Unsupported Media Type",
            422 => "Unprocessable Entity",
            408 => "Request Timeout",
            500 => "Internal Server Error",
            _ => "OK",
        }
    }
}

fn object_type_name(object_type: &ObjectType) -> &'static str {
    match object_type {
        ObjectType::Path(_) => "path",
        ObjectType::Rectangle { .. } => "rect",
        ObjectType::Ellipse { .. } => "ellipse",
        ObjectType::Star { .. } => "star",
        ObjectType::Polygon { .. } => "polygon",
        ObjectType::Line { .. } => "line",
        ObjectType::Text { .. } => "text",
        ObjectType::Group(_) => "group",
        ObjectType::ClippingMask { .. } => "clipping-mask",
        ObjectType::Use { .. } => "use",
        ObjectType::Image { .. } => "image",
    }
}

fn transform_json(transform: &Transform) -> Value {
    json!({
        "x": transform.x,
        "y": transform.y,
        "rotation": transform.rotation,
        "scale_x": transform.scale_x,
        "scale_y": transform.scale_y,
    })
}

fn object_json(obj: &Object) -> Value {
    let bbox = obj.bounding_box().map(|(min, max)| {
        json!({ "min_x": min.x, "min_y": min.y, "max_x": max.x, "max_y": max.y })
    });
    json!({
        "id": obj.id,
        "name": obj.name,
        "type": object_type_name(&obj.object_type),
        "visible": obj.visible,
        "locked": obj.locked,
        "opacity": obj.opacity,
        "blend_mode": format!("{:?}", obj.blend_mode),
        "transform": transform_json(&obj.transform),
        "bbox": bbox,
    })
}

/// `?key=value` lookup with percent-decoding (values are UTF-8, decoded
/// lossily — a malformed escape is not worth a 400 for a file path).
fn query_param(query: &str, key: &str) -> Option<String> {
    for pair in query.split('&') {
        let Some((k, v)) = pair.split_once('=') else {
            continue;
        };
        if k.trim() == key {
            return Some(percent_decode(v.trim()));
        }
    }
    None
}

fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

/// Parse a JSON request body, mapping malformed payloads to a 400.
fn parse_body_json(body: &[u8]) -> Result<Value, ApiResponse> {
    if body.is_empty() {
        // An empty body is a valid "all defaults" request.
        return Ok(Value::Object(serde_json::Map::new()));
    }
    serde_json::from_slice(body)
        .map_err(|e| ApiResponse::error(400, format!("Invalid JSON body: {e}")))
}

fn json_number(value: &Value, keys: &[&str], default: f64) -> f64 {
    for key in keys {
        if let Some(found) = value.get(*key) {
            if let Some(n) = found.as_f64() {
                return n;
            }
            if let Some(s) = found.as_str() {
                if let Ok(n) = s.trim().parse::<f64>() {
                    return n;
                }
            }
        }
    }
    default
}

fn json_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(|v| v.as_str()).map(String::from))
}

fn json_bool(value: &Value, keys: &[&str], default: bool) -> bool {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(|v| v.as_bool()))
        .unwrap_or(default)
}

/// Optional `"fill": "#ff8800"` / `"fill": [r,g,b,a]` on create requests.
fn json_fill(value: &Value) -> Option<FillStyle> {
    let fill = value.get("fill")?;
    if let Some(text) = fill.as_str() {
        return parse_svg_color(text).map(FillStyle::solid);
    }
    let arr = fill.as_array()?;
    if arr.len() < 3 {
        return None;
    }
    let channel = |idx: usize, default: f32| -> f32 {
        arr.get(idx)
            .and_then(|v| v.as_f64())
            .map(|v| (v as f32).clamp(0.0, 1.0))
            .unwrap_or(default)
    };
    Some(FillStyle::solid([
        channel(0, 0.0),
        channel(1, 0.0),
        channel(2, 0.0),
        channel(3, 1.0),
    ]))
}

/// Reject NaN/negative geometry before it reaches the document (a `0x0` or
/// `NaN` object is invisible and poisons bounding boxes downstream).
fn validate_size(label: &str, value: f64) -> Result<f64, ApiResponse> {
    if !value.is_finite() || value <= 0.0 {
        return Err(ApiResponse::error(
            422,
            format!("{label} must be a finite number greater than 0 (got {value})"),
        ));
    }
    Ok(value)
}

fn doc_summary(api: &ApiState) -> Value {
    json!({
        "name": api.document.name,
        "width": api.document.width,
        "height": api.document.height,
        "color_mode": api.document.color_mode.to_string(),
        "active_layer": api.document.active_layer_idx,
        "object_count": api.document.all_objects().count(),
        "symbol_count": api.document.symbols.len(),
        "artboards": api.document.artboards.iter().map(|a| json!({
            "id": a.id,
            "name": a.name,
            "x": a.x,
            "y": a.y,
            "width": a.width,
            "height": a.height,
        })).collect::<Vec<_>>(),
        "layers": api.document.layers.iter().enumerate().map(|(idx, layer)| json!({
            "index": idx,
            "id": layer.id,
            "name": layer.name,
            "visible": layer.visible,
            "locked": layer.locked,
            "opacity": layer.opacity,
            "object_count": layer.objects.len(),
        })).collect::<Vec<_>>(),
    })
}

const ROUTE_INDEX: &str = "\
Amata local API
  GET  /api/health
  GET  /api/document
  GET  /api/objects
  POST /api/objects/rect      { name?, x?, y?, width|w, height|h, corner_radius?, fill? }
  POST /api/objects/ellipse   { name?, cx|x?, cy|y?, rx, ry?, fill? }
  POST /api/objects/path      { name?, points: [[x, y], ...], closed?, fill? }
  POST /api/script            { script: \"...\" }  (or a raw Rhai body)
  POST /api/export/svg        [ ?path=/tmp/out.svg ]  (no path = inline SVG )
";

/// Dispatch one request. `target` is the raw request target, so it may carry a
/// query string.
pub fn route(api: &mut ApiState, method: &str, target: &str, body: &[u8]) -> ApiResponse {
    api.request_count += 1;
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (target, ""),
    };
    let path = path.trim_end_matches('/');
    let path = if path.is_empty() { "/" } else { path };
    let method = method.to_ascii_uppercase();

    match (method.as_str(), path) {
        ("GET", "/") => ApiResponse::text(200, ROUTE_INDEX),
        ("GET", "/api/health") => ApiResponse::json(
            200,
            &json!({
                "status": "ok",
                "app": "amata",
                "version": env!("CARGO_PKG_VERSION"),
                "requests": api.request_count,
            }),
        ),
        ("GET", "/api/document") => ApiResponse::json(200, &doc_summary(api)),
        ("GET", "/api/objects") => {
            let objects: Vec<Value> = api
                .document
                .all_objects()
                .map(|(layer_idx, obj)| {
                    let mut value = object_json(obj);
                    if let Some(map) = value.as_object_mut() {
                        map.insert("layer".to_string(), json!(layer_idx));
                    }
                    value
                })
                .collect();
            ApiResponse::json(200, &json!({ "count": objects.len(), "objects": objects }))
        }
        ("POST", "/api/objects/rect") => create_rect(api, body),
        ("POST", "/api/objects/ellipse") => create_ellipse(api, body),
        ("POST", "/api/objects/path") => create_path(api, body),
        ("POST", "/api/script") => run_script(api, body),
        ("POST", "/api/export/svg") => export_svg_route(api, query),
        _ => {
            let known = matches!(
                path,
                "/" | "/api/health"
                    | "/api/document"
                    | "/api/objects"
                    | "/api/objects/rect"
                    | "/api/objects/ellipse"
                    | "/api/objects/path"
                    | "/api/script"
                    | "/api/export/svg"
            );
            if known {
                ApiResponse::error(405, format!("{path} does not accept {method}"))
            } else {
                ApiResponse::error(404, format!("Unknown route: {method} {path}"))
            }
        }
    }
}

fn object_created(api: &ApiState, obj: &Object) -> ApiResponse {
    ApiResponse::json(
        200,
        &json!({
            "ok": true,
            "object": object_json(obj),
            "object_count": api.document.all_objects().count() + 1,
        }),
    )
}

fn create_rect(api: &mut ApiState, body: &[u8]) -> ApiResponse {
    let payload = match parse_body_json(body) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let width = match validate_size("width", json_number(&payload, &["width", "w"], 100.0)) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let height = match validate_size("height", json_number(&payload, &["height", "h"], 100.0)) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let corner_radius = json_number(&payload, &["corner_radius", "radius"], 0.0)
        .max(0.0)
        .min(width.min(height) / 2.0);
    let x = json_number(&payload, &["x"], 0.0);
    let y = json_number(&payload, &["y"], 0.0);
    if !x.is_finite() || !y.is_finite() {
        return ApiResponse::error(422, "x/y must be finite numbers");
    }
    let name = json_string(&payload, &["name"]).unwrap_or_else(|| "Rectangle".to_string());

    let mut obj = Object::new_rect(&name, x, y, width, height, corner_radius);
    if let Some(fill) = json_fill(&payload) {
        obj.fill = Some(fill);
    }
    let response = object_created(api, &obj);
    api.document.add_object(obj);
    response
}

fn create_ellipse(api: &mut ApiState, body: &[u8]) -> ApiResponse {
    let payload = match parse_body_json(body) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let rx = match validate_size("rx", json_number(&payload, &["rx", "radius"], 50.0)) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let ry = match validate_size("ry", json_number(&payload, &["ry", "radius"], rx)) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let cx = json_number(&payload, &["cx", "x"], 0.0);
    let cy = json_number(&payload, &["cy", "y"], 0.0);
    if !cx.is_finite() || !cy.is_finite() {
        return ApiResponse::error(422, "cx/cy must be finite numbers");
    }
    let name = json_string(&payload, &["name"]).unwrap_or_else(|| "Ellipse".to_string());

    let mut obj = Object::new_ellipse(&name, cx, cy, rx, ry);
    if let Some(fill) = json_fill(&payload) {
        obj.fill = Some(fill);
    }
    let response = object_created(api, &obj);
    api.document.add_object(obj);
    response
}



fn create_path(api: &mut ApiState, body: &[u8]) -> ApiResponse {
    let payload = match parse_body_json(body) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(points) = payload.get("points").and_then(|v| v.as_array()) else {
        return ApiResponse::error(422, "`points` must be an array of [x, y] pairs");
    };
    if points.len() < 2 {
        return ApiResponse::error(422, "`points` needs at least 2 vertices");
    }
    let mut vertices = Vec::with_capacity(points.len());
    for (idx, point) in points.iter().enumerate() {
        // Accepts [x, y] and (for convenience) {"x": .., "y": ..}.
        let (x, y) = match point.as_array() {
            Some(pair) if pair.len() >= 2 => (
                pair[0].as_f64().unwrap_or(f64::NAN),
                pair[1].as_f64().unwrap_or(f64::NAN),
            ),
            _ => match point.as_object() {
                Some(map) => (
                    map.get("x").and_then(|v| v.as_f64()).unwrap_or(f64::NAN),
                    map.get("y").and_then(|v| v.as_f64()).unwrap_or(f64::NAN),
                ),
                None => (f64::NAN, f64::NAN),
            },
        };
        if !x.is_finite() || !y.is_finite() {
            return ApiResponse::error(422, format!("points[{idx}] is not a finite [x, y] pair"));
        }
        vertices.push(AnchorPoint::new(x, y));
    }
    let closed = json_bool(&payload, &["closed"], true);
    let name = json_string(&payload, &["name"]).unwrap_or_else(|| "Path".to_string());

    let mut path = PathData::from_polygon_points(&vertices, closed);
    if let Some(fill) = json_fill(&payload) {
        path.fill = Some(fill);
    }
    let obj = Object::new_path(&name, path);
    let response = object_created(api, &obj);
    api.document.add_object(obj);
    response
}

fn run_script(api: &mut ApiState, body: &[u8]) -> ApiResponse {
    // Prefer {"script": "..."} but fall back to a raw Rhai body so that
    // `curl --data-binary @plugin.rhai` works too.
    let script = match parse_body_json(body) {
        Ok(payload) => match json_string(&payload, &["script", "source", "code"]) {
            Some(script) => script,
            None if !body.is_empty() => String::from_utf8_lossy(body).to_string(),
            None => return ApiResponse::error(422, "`script` must be a string"),
        },
        Err(_) => String::from_utf8_lossy(body).to_string(),
    };
    if script.trim().is_empty() {
        return ApiResponse::error(422, "script is empty");
    }

    let before = api.document.all_objects().count();
    let engine = ScriptEngine::new();
    // Scripts receive a scratch `AppState`; the document is what they mutate.
    let mut scratch_state = AppState::default();
    match engine.run_script(&script, &mut api.document, &mut scratch_state) {
        Ok(_) => {
            // Scripts may replace layers wholesale; restore invariants.
            api.document.normalize();
            let after = api.document.all_objects().count();
            ApiResponse::json(
                200,
                &json!({
                    "ok": true,
                    "objects_before": before,
                    "objects_after": after,
                    "document": doc_summary(api),
                }),
            )
        }
        Err(err) => ApiResponse::error(422, err),
    }
}

fn export_svg_route(api: &mut ApiState, query: &str) -> ApiResponse {
    let svg = crate::io::svg::export_svg(&api.document);
    match query_param(query, "path") {
        Some(raw_path) => {
            let path = PathBuf::from(raw_path);
            if path.is_dir() {
                return ApiResponse::error(422, "path points at a directory");
            }
            match crate::io::atomic::atomic_write_str(&path, &svg) {
                Ok(()) => ApiResponse::json(
                    200,
                    &json!({
                        "ok": true,
                        "path": path.display().to_string(),
                        "bytes": svg.len(),
                    }),
                ),
                Err(err) => ApiResponse::error(500, format!("Could not write SVG: {err}")),
            }
        }
        None => ApiResponse::bytes(200, "image/svg+xml; charset=utf-8", svg.into_bytes()),
    }
}


/// A parsed HTTP request line + body.
struct Request {
    method: String,
    target: String,
    body: Vec<u8>,
}

fn header_end(raw: &[u8]) -> Option<usize> {
    raw.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
}

/// Read one HTTP/1.1 request from `stream`. Fails with `InvalidData` on
/// malformed input; chunked transfer encoding is refused instead of guessed.
fn read_request(stream: &mut TcpStream) -> std::io::Result<Request> {
    let mut raw: Vec<u8> = Vec::with_capacity(1024);
    let mut chunk = [0u8; 4096];
    let head_len = loop {
        if let Some(end) = header_end(&raw) {
            break end;
        }
        if raw.len() > MAX_HEADER_BYTES {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "request headers too large",
            ));
        }
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Err(std::io::Error::new(
                ErrorKind::UnexpectedEof,
                "connection closed before headers completed",
            ));
        }
        raw.extend_from_slice(&chunk[..read]);
    };

    let head = String::from_utf8_lossy(&raw[..head_len]).to_string();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or_default().to_string();
    if method.is_empty() || target.is_empty() {
        return Err(std::io::Error::new(
            ErrorKind::InvalidData,
            "malformed request line",
        ));
    }

    let mut content_length = 0usize;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        match name.as_str() {
            "content-length" => {
                content_length = value.parse::<usize>().map_err(|_| {
                    std::io::Error::new(ErrorKind::InvalidData, "invalid Content-Length")
                })?;
            }
            "transfer-encoding" if value.to_ascii_lowercase().contains("chunked") => {
                return Err(std::io::Error::new(
                    ErrorKind::InvalidData,
                    "chunked transfer encoding is not supported",
                ));
            }
            _ => {}
        }
    }

    if content_length > MAX_BODY_BYTES {
        return Err(std::io::Error::new(
            ErrorKind::InvalidData,
            "request body too large",
        ));
    }

    let mut body = raw[head_len..].to_vec();
    while body.len() < content_length {
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);

    Ok(Request {
        method,
        target,
        body,
    })
}



fn write_response(stream: &mut TcpStream, response: &ApiResponse) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {} {}\r\n\
         Content-Type: {}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         Access-Control-Allow-Origin: *\r\n\
         Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
         Access-Control-Allow-Headers: Content-Type\r\n\
         \r\n",
        response.status,
        response.status_text(),
        response.content_type,
        response.body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(&response.body)?;
    stream.flush()
}

/// Discard any bytes the peer sent after we stopped parsing (an early-rejected
/// chunked body, a pipelined request, ...). Closing a socket that still has
/// unread data makes the OS answer with RST instead of FIN, which clients
/// surface as "connection reset" even though they already got our response.
fn drain_pending_request_bytes(stream: &mut TcpStream) {
    const DRAIN_CAP: usize = 64 * 1024;
    let _ = stream.set_read_timeout(Some(Duration::from_millis(150)));
    let mut scratch = [0u8; 8192];
    let mut drained = 0usize;
    while drained < DRAIN_CAP {
        match stream.read(&mut scratch) {
            Ok(0) => break,
            Ok(read) => drained += read,
            Err(_) => break,
        }
    }
}

/// Serve one connection: parse, route, answer. Errors are reported to the
/// client when possible and otherwise swallowed (a broken peer must not take
/// the server down).
pub fn handle_connection(mut stream: TcpStream, api: &mut ApiState) {
    // The listener is non-blocking so the loop can notice `stop`; on BSD/macOS
    // accepted sockets inherit O_NONBLOCK, which would make the very first
    // `read` fail with EAGAIN before the client has sent anything.
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));

    match read_request(&mut stream) {
        Ok(request) => {
            if request.method.eq_ignore_ascii_case("OPTIONS") {
                let response = ApiResponse::text(200, "");
                let _ = write_response(&mut stream, &response);
                drain_pending_request_bytes(&mut stream);
                return;
            }
            let response = route(api, &request.method, &request.target, &request.body);
            let _ = write_response(&mut stream, &response);
        }
        Err(err) => {
            let status = match err.kind() {
                ErrorKind::WouldBlock | ErrorKind::TimedOut => 408,
                _ if err.to_string().contains("too large") => 413,
                _ => 400,
            };
            let response = ApiResponse::error(status, err.to_string());
            let _ = write_response(&mut stream, &response);
        }
    }
    drain_pending_request_bytes(&mut stream);
}

/// Accept connections until `stop` is set. Requests are handled on the calling
/// thread; plugin scripts are resource-bounded, so a slow script can only delay
/// subsequent requests, never wedge the process.
pub fn serve(listener: TcpListener, mut api: ApiState, stop: &AtomicBool) -> std::io::Result<()> {
    listener.set_nonblocking(true)?;
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _peer)) => handle_connection(stream, &mut api),
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

/// `amata serve` entry point.
pub fn handle_serve(port: u16, input: Option<PathBuf>) -> Result<bool, Box<dyn std::error::Error>> {
    let document = match input {
        Some(path) => {
            println!("📂 Loading '{:?}'...", path);
            load_any_document(&path)?
        }
        None => Document::default(),
    };

    // Loopback only: this is a local integration surface, not a public service.
    let addr = format!("127.0.0.1:{port}");
    let listener = match TcpListener::bind(&addr) {
        Ok(listener) => listener,
        Err(err) => {
            return Err(format!("Could not bind {addr}: {err} (is the port already in use?)").into())
        }
    };

    println!("🌐 Amata API server listening on http://{addr}");
    println!(
        "   Document: '{}' ({} × {} px, {} object(s))",
        document.name,
        document.width,
        document.height,
        document.all_objects().count()
    );
    println!("   Endpoints:");
    for route_line in ROUTE_INDEX.lines() {
        println!("     {route_line}");
    }
    println!("   Stop with Ctrl+C.");

    let stop = AtomicBool::new(false);
    serve(listener, ApiState::new(document), &stop)?;
    Ok(false)
}

