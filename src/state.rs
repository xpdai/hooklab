//! 共用狀態：攔截到的請求環狀緩衝 + 轉發設定。

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use serde::Serialize;

/// 一筆被攔截下來的 HTTP 請求。
#[derive(Clone, Serialize)]
pub struct CapturedRequest {
    pub id: u64,
    pub timestamp_ms: u64,
    pub method: String,
    pub path: String,
    pub query: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub body_is_binary: bool,
}

/// 轉發目標設定。
pub struct Config {
    pub target: Option<String>,
    pub auto_forward: bool,
}

/// 全域 app 狀態，包在 Arc 裡給 axum 共用。
pub struct AppState {
    pub requests: Mutex<VecDeque<CapturedRequest>>,
    pub config: Mutex<Config>,
    counter: AtomicU64,
    capacity: usize,
}

impl AppState {
    pub fn new(target: Option<String>, capacity: usize) -> Self {
        AppState {
            requests: Mutex::new(VecDeque::new()),
            config: Mutex::new(Config {
                target,
                auto_forward: false,
            }),
            counter: AtomicU64::new(1),
            capacity,
        }
    }

    pub fn next_id(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }

    /// 新請求放最前面，超過容量就丟掉最舊的。
    pub fn push(&self, req: CapturedRequest) {
        let mut q = self.requests.lock().unwrap();
        q.push_front(req);
        while q.len() > self.capacity {
            q.pop_back();
        }
    }

    pub fn list(&self) -> Vec<CapturedRequest> {
        self.requests.lock().unwrap().iter().cloned().collect()
    }

    pub fn get(&self, id: u64) -> Option<CapturedRequest> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .find(|r| r.id == id)
            .cloned()
    }

    pub fn clear(&self) {
        self.requests.lock().unwrap().clear();
    }
}
