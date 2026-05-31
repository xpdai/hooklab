# 🪝 hooklab

[![CI](https://github.com/xpdai/hooklab/actions/workflows/ci.yml/badge.svg)](https://github.com/xpdai/hooklab/actions/workflows/ci.yml)

本地 webhook 攔截 / 檢視 / 轉發 / 重放工具。單一 Rust 執行檔，零外部依賴、零雲端。

開發 webhook（金流回呼、第三方通知）時的痛點：對方打的是公開網址、你的程式在 localhost、而且事件不能隨便重來。hooklab 把整個除錯流程包成一個本地工具。

## 功能

- **攔截**：任何打進來的 HTTP 請求（任意 method / 路徑）全部記錄 —— method、path、query、headers、body。
- **檢視**：內建 Web UI，即時列表 + 詳情。
- **轉發 / 重放**：一鍵把攔到的請求轉發到你的 localhost，看回應；同一個請求想打幾次打幾次。
- **編輯後送出**：改 method / headers / body 再送，測你的程式各種情況（例如把「付款成功」改成「金額 0」）。
- **auto-forward（穿透代理）**：開啟後，每個攔到的請求自動轉發到 target，並把回應原樣回給來源。

## 建置

```sh
cargo build --release
# 產物：target/release/hooklab.exe
```

## 使用

```sh
hooklab --port 4500 --target http://localhost:3000
```

| 選項 | 說明 | 預設 |
|------|------|------|
| `-p, --port <PORT>` | 監聽 port | `4500` |
| `-t, --target <URL>` | 轉發目標 | 無（可在 UI 設定） |
| `-s, --store <FILE>` | 把攔截的請求持久化到 JSONL 檔，重啟自動載回 | 無（純記憶體） |

啟動後：

- **UI**：<http://localhost:4500/__hooklab>
- **攔截端點**：把 webhook 送到 `http://localhost:4500/<任意路徑>`

`target` 與 auto-forward 也可在 UI 上即時調整，不必重啟。

## 接真實第三方 webhook（Stripe / 綠界 / LINE…）

真實的 webhook 來源在公網，打不到你的 `localhost`。hooklab 本身就是個普通 HTTP server，
**前面套任何穿透工具就能收公網 webhook**，不需要它內建 relay：

```sh
# 1. 跑 hooklab
hooklab --port 4500 --target http://localhost:3000

# 2. 另開一個視窗，用 cloudflared（或 ngrok）把它暴露到公網
cloudflared tunnel --url http://localhost:4500
#   → 會給你一個 https://xxxx.trycloudflare.com 網址

# 3. 把第三方平台的 webhook 設成 https://xxxx.trycloudflare.com/<你的路徑>
```

之後所有公網 webhook 都會進 hooklab，可即時檢視、重放、編輯重送，
開 auto-forward 還能直接代理到你正在開發的 localhost 服務。

## API

| Method | 路徑 | 說明 |
|--------|------|------|
| `GET` | `/__hooklab/api/requests` | 列出攔截到的請求（新到舊） |
| `GET` | `/__hooklab/api/requests/:id` | 單筆詳情 |
| `DELETE` | `/__hooklab/api/requests` | 清空 |
| `POST` | `/__hooklab/api/requests/:id/forward` | 把某筆轉發到 target |
| `POST` | `/__hooklab/api/send` | 自訂 / 編輯後送出 |
| `GET` / `POST` | `/__hooklab/api/config` | 讀取 / 設定 target 與 auto-forward |

## 已知限制（MVP）

- 記憶體保留最近 500 筆（環狀緩衝）；加 `--store` 可持久化到磁碟、重啟自動載回。
- target 走 HTTP（轉發給 localhost 用，未編譯 TLS，不支援 https target）。
- binary body 只記錄大小，不重放原始 bytes。
- 保留路徑 `/__hooklab` 不會被攔截 —— 若你的 webhook 剛好用到這個前綴需另外處理。
