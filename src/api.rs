//! `/__hooklab` 底下的 UI 與 JSON API。

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::forward::{forward, ForwardResult};
use crate::state::{AppState, CapturedRequest};

pub async fn ui() -> Html<&'static str> {
    Html(include_str!("assets/index.html"))
}

pub async fn list(State(s): State<Arc<AppState>>) -> Json<Vec<CapturedRequest>> {
    Json(s.list())
}

pub async fn detail(
    State(s): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> Result<Json<CapturedRequest>, StatusCode> {
    s.get(id).map(Json).ok_or(StatusCode::NOT_FOUND)
}

pub async fn clear(State(s): State<Arc<AppState>>) -> StatusCode {
    s.clear();
    StatusCode::NO_CONTENT
}

#[derive(Serialize)]
pub struct ConfigView {
    pub target: Option<String>,
    pub auto_forward: bool,
}

pub async fn get_config(State(s): State<Arc<AppState>>) -> Json<ConfigView> {
    let c = s.config.lock().unwrap();
    Json(ConfigView {
        target: c.target.clone(),
        auto_forward: c.auto_forward,
    })
}

#[derive(Deserialize)]
pub struct ConfigUpdate {
    pub target: Option<String>,
    pub auto_forward: Option<bool>,
}

pub async fn set_config(
    State(s): State<Arc<AppState>>,
    Json(u): Json<ConfigUpdate>,
) -> Json<ConfigView> {
    let mut c = s.config.lock().unwrap();
    c.target = u.target.filter(|t| !t.trim().is_empty());
    if let Some(af) = u.auto_forward {
        c.auto_forward = af;
    }
    Json(ConfigView {
        target: c.target.clone(),
        auto_forward: c.auto_forward,
    })
}

/// 把已攔截的某筆請求轉發到 target。
pub async fn forward_captured(
    State(s): State<Arc<AppState>>,
    Path(id): Path<u64>,
) -> Json<ForwardResult> {
    let req = match s.get(id) {
        Some(r) => r,
        None => return Json(ForwardResult::err("找不到該請求")),
    };
    let target = { s.config.lock().unwrap().target.clone() };
    let Some(target) = target else {
        return Json(ForwardResult::err("尚未設定 target"));
    };
    let pq = if req.query.is_empty() {
        req.path.clone()
    } else {
        format!("{}?{}", req.path, req.query)
    };
    Json(forward(&target, &req.method, &pq, &req.headers, req.body.into_bytes()).await)
}

#[derive(Deserialize)]
pub struct SendRequest {
    pub method: String,
    pub path: String,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub target: Option<String>,
}

/// 自訂 / 編輯後送出一個請求。
pub async fn send(
    State(s): State<Arc<AppState>>,
    Json(r): Json<SendRequest>,
) -> Json<ForwardResult> {
    let target = r
        .target
        .filter(|t| !t.trim().is_empty())
        .or_else(|| s.config.lock().unwrap().target.clone());
    let Some(target) = target else {
        return Json(ForwardResult::err("尚未設定 target"));
    };
    Json(forward(&target, &r.method, &r.path, &r.headers, r.body.into_bytes()).await)
}
