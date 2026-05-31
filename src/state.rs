//! 共用狀態：攔截到的請求環狀緩衝 + 轉發設定 + 選用的磁碟持久化。

use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// 一筆被攔截下來的 HTTP 請求。
#[derive(Clone, Serialize, Deserialize)]
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
    /// 設定後，攔截到的請求會以 JSONL 持久化到這個檔，重啟時自動載回。
    store_path: Option<PathBuf>,
    store_file: Mutex<Option<File>>,
}

impl AppState {
    pub fn new(target: Option<String>, capacity: usize, store_path: Option<PathBuf>) -> Self {
        let mut deque = VecDeque::new();
        let mut max_id = 0u64;

        // 從 JSONL 載回先前的請求（檔案是 oldest→newest，push_front 後 front=newest）。
        if let Some(p) = &store_path {
            if let Ok(content) = std::fs::read_to_string(p) {
                for line in content.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(req) = serde_json::from_str::<CapturedRequest>(line) {
                        max_id = max_id.max(req.id);
                        deque.push_front(req);
                        while deque.len() > capacity {
                            deque.pop_back();
                        }
                    }
                }
            }
        }

        let store_file = store_path
            .as_ref()
            .and_then(|p| OpenOptions::new().create(true).append(true).open(p).ok());

        AppState {
            requests: Mutex::new(deque),
            config: Mutex::new(Config {
                target,
                auto_forward: false,
            }),
            counter: AtomicU64::new(max_id + 1),
            capacity,
            store_path,
            store_file: Mutex::new(store_file),
        }
    }

    pub fn next_id(&self) -> u64 {
        self.counter.fetch_add(1, Ordering::SeqCst)
    }

    /// 新請求放最前面，超過容量就丟掉最舊的；有設 store 就同步 append 一行。
    pub fn push(&self, req: CapturedRequest) {
        if let Ok(mut guard) = self.store_file.lock() {
            if let Some(f) = guard.as_mut() {
                if let Ok(line) = serde_json::to_string(&req) {
                    let _ = writeln!(f, "{line}");
                    let _ = f.flush();
                }
            }
        }
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
        // 連磁碟上的 store 一起清空，並重開 append handle。
        if let Some(p) = &self.store_path {
            let _ = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(p);
            if let Ok(mut guard) = self.store_file.lock() {
                *guard = OpenOptions::new().create(true).append(true).open(p).ok();
            }
        }
    }
}
