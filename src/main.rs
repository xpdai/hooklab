//! hooklab — 本地 webhook 攔截 / 檢視 / 轉發 / 重放工具。

mod api;
mod capture;
mod forward;
mod state;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use state::AppState;

#[tokio::main]
async fn main() {
    let mut port: u16 = 4500;
    let mut target: Option<String> = None;
    let mut store: Option<std::path::PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--port" | "-p" => {
                if let Some(v) = args.next() {
                    port = v.parse().unwrap_or(port);
                }
            }
            "--target" | "-t" => target = args.next(),
            "--store" | "-s" => store = args.next().map(std::path::PathBuf::from),
            "--help" | "-h" => {
                print_help();
                return;
            }
            _ => {}
        }
    }

    let state = Arc::new(AppState::new(target.clone(), 500, store.clone()));

    let app = Router::new()
        .route("/__hooklab", get(api::ui))
        .route("/__hooklab/", get(api::ui))
        .route("/__hooklab/api/requests", get(api::list).delete(api::clear))
        .route("/__hooklab/api/requests/:id", get(api::detail))
        .route(
            "/__hooklab/api/requests/:id/forward",
            post(api::forward_captured),
        )
        .route("/__hooklab/api/send", post(api::send))
        .route(
            "/__hooklab/api/config",
            get(api::get_config).post(api::set_config),
        )
        .fallback(capture::capture)
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("無法綁定 port");

    println!("hooklab 啟動於 http://localhost:{port}");
    println!("  UI：     http://localhost:{port}/__hooklab");
    println!("  攔截：   把任何 webhook 送到 http://localhost:{port}/<任意路徑>");
    if let Some(t) = &target {
        println!("  target： {t}");
    }
    if let Some(s) = &store {
        println!("  store：  {}（重啟自動載回）", s.display());
    }

    axum::serve(listener, app).await.expect("server 啟動失敗");
}

fn print_help() {
    println!(
        "hooklab — 本地 webhook 攔截 / 轉發 / 重放工具\n\n\
         用法：\n\
         \x20 hooklab [--port <PORT>] [--target <URL>] [--store <FILE>]\n\n\
         選項：\n\
         \x20 -p, --port    <PORT>   監聽 port（預設 4500）\n\
         \x20 -t, --target  <URL>    轉發目標，例如 http://localhost:3000\n\
         \x20 -s, --store   <FILE>   把攔截的請求持久化到 JSONL 檔，重啟自動載回\n\
         \x20 -h, --help             顯示說明\n\n\
         啟動後開 http://localhost:<PORT>/__hooklab 看 UI。"
    );
}
