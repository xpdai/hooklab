//! Catch-all handler：記錄任何打進來的請求，必要時自動轉發。

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    body::{Body, Bytes},
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
};

use crate::forward::forward;
use crate::state::{AppState, CapturedRequest};

pub async fn capture(
    State(s): State<Arc<AppState>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let header_vec: Vec<(String, String)> = headers
        .iter()
        .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("<binary>").to_string()))
        .collect();

    let (body_str, is_bin) = match std::str::from_utf8(&body) {
        Ok(s) => (s.to_string(), false),
        Err(_) => (format!("<{} bytes binary>", body.len()), true),
    };

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let query = uri.query().unwrap_or("").to_string();

    s.push(CapturedRequest {
        id: s.next_id(),
        timestamp_ms: ts,
        method: method.as_str().to_string(),
        path: uri.path().to_string(),
        query: query.clone(),
        headers: header_vec.clone(),
        body: body_str,
        body_is_binary: is_bin,
    });

    // auto-forward：當穿透代理用，把回應原樣回給來源。
    let (auto, target) = {
        let c = s.config.lock().unwrap();
        (c.auto_forward, c.target.clone())
    };
    if auto {
        if let Some(target) = target {
            let pq = if query.is_empty() {
                uri.path().to_string()
            } else {
                format!("{}?{}", uri.path(), query)
            };
            let res = forward(&target, method.as_str(), &pq, &header_vec, body.to_vec()).await;
            if let Some(status) = res.status {
                let mut builder = Response::builder().status(status);
                for (k, v) in &res.headers {
                    let lk = k.to_lowercase();
                    if lk == "transfer-encoding" || lk == "content-length" || lk == "connection" {
                        continue;
                    }
                    builder = builder.header(k.as_str(), v.as_str());
                }
                return builder
                    .body(Body::from(res.body))
                    .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response());
            }
            return (StatusCode::BAD_GATEWAY, res.error.unwrap_or_default()).into_response();
        }
    }

    (StatusCode::OK, "captured by hooklab\n").into_response()
}
