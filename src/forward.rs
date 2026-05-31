//! 把一個請求轉發到 target，回傳結果（給 UI 顯示）。

use serde::Serialize;

#[derive(Serialize)]
pub struct ForwardResult {
    pub status: Option<u16>,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub error: Option<String>,
    pub url: String,
}

impl ForwardResult {
    pub fn err(msg: &str) -> Self {
        ForwardResult {
            status: None,
            headers: vec![],
            body: String::new(),
            error: Some(msg.to_string()),
            url: String::new(),
        }
    }
}

/// 轉發。`path_and_query` 應以 `/` 開頭（沒有的話會補上）。
pub async fn forward(
    base: &str,
    method: &str,
    path_and_query: &str,
    headers: &[(String, String)],
    body: Vec<u8>,
) -> ForwardResult {
    let base = base.trim_end_matches('/');
    let pq = if path_and_query.starts_with('/') {
        path_and_query.to_string()
    } else {
        format!("/{path_and_query}")
    };
    let url = format!("{base}{pq}");

    let client = reqwest::Client::new();
    let m = reqwest::Method::from_bytes(method.as_bytes()).unwrap_or(reqwest::Method::POST);
    let mut rb = client.request(m, url.as_str());

    for (k, v) in headers {
        let lk = k.to_lowercase();
        // 這些 header 交給 client 自己算，不要硬轉發避免衝突。
        if lk == "host" || lk == "content-length" || lk == "connection" {
            continue;
        }
        rb = rb.header(k.as_str(), v.as_str());
    }
    rb = rb.body(body);

    match rb.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let headers = resp
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
                .collect();
            let body = match resp.text().await {
                Ok(t) => t,
                Err(e) => format!("<讀取回應 body 失敗: {e}>"),
            };
            ForwardResult {
                status: Some(status),
                headers,
                body,
                error: None,
                url,
            }
        }
        Err(e) => ForwardResult {
            status: None,
            headers: vec![],
            body: String::new(),
            error: Some(e.to_string()),
            url,
        },
    }
}
