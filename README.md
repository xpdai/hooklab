# 🪝 hooklab

[![CI](https://github.com/xpdai/hooklab/actions/workflows/ci.yml/badge.svg)](https://github.com/xpdai/hooklab/actions/workflows/ci.yml)

**English** | [繁體中文](README.zh-TW.md)

A local webhook capture / inspect / forward / replay tool. Single Rust binary, no external services, no cloud.

Developing against webhooks (payment callbacks, third-party notifications) is painful: the sender hits a public URL, your code runs on localhost, and you can't just replay events at will. hooklab wraps that whole debugging loop into one local tool.

## Features

- **Capture** — records every incoming HTTP request (any method / path): method, path, query, headers, body.
- **Inspect** — built-in web UI with a live list and a detail view.
- **Forward / replay** — one click forwards a captured request to your localhost and shows the response; replay the same request as many times as you want.
- **Edit & send** — tweak method / headers / body and resend to exercise every code path (e.g. flip a "payment succeeded" into "amount 0").
- **auto-forward (transparent proxy)** — when on, every captured request is forwarded to the target automatically and the response is passed straight back to the caller.

## Build

```sh
cargo build --release
# output: target/release/hooklab (hooklab.exe on Windows)
```

## Usage

```sh
hooklab --port 4500 --target http://localhost:3000
```

| Option | Description | Default |
|--------|-------------|---------|
| `-p, --port <PORT>` | Listen port | `4500` |
| `-t, --target <URL>` | Forward target | none (settable in the UI) |
| `-s, --store <FILE>` | Persist captured requests to a JSONL file, reloaded on restart | none (in-memory only) |

Once running:

- **UI**: <http://localhost:4500/__hooklab>
- **Capture endpoint**: send webhooks to `http://localhost:4500/<any-path>`

`target` and auto-forward can also be changed live in the UI without restarting.

## Receiving real third-party webhooks (Stripe / LINE / …)

Real webhook senders live on the public internet and can't reach your `localhost`.
hooklab is just a plain HTTP server, so **putting any tunnel in front of it lets you
receive public webhooks** — no built-in relay required:

```sh
# 1. Run hooklab
hooklab --port 4500 --target http://localhost:3000

# 2. In another shell, expose it with cloudflared (or ngrok)
cloudflared tunnel --url http://localhost:4500
#   → gives you a https://xxxx.trycloudflare.com URL

# 3. Point the provider's webhook at https://xxxx.trycloudflare.com/<your-path>
```

Every public webhook now lands in hooklab — inspect, replay, edit & resend live,
and with auto-forward on it proxies straight through to your local service.

## API

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/__hooklab/api/requests` | List captured requests (newest first) |
| `GET` | `/__hooklab/api/requests/:id` | Single request detail |
| `DELETE` | `/__hooklab/api/requests` | Clear all |
| `POST` | `/__hooklab/api/requests/:id/forward` | Forward a captured request to the target |
| `POST` | `/__hooklab/api/send` | Custom / edited send |
| `GET` / `POST` | `/__hooklab/api/config` | Get / set target and auto-forward |

## Known limitations (MVP)

- Keeps the most recent 500 requests in memory (ring buffer); pass `--store` to persist to disk and reload on restart.
- The target uses plain HTTP (intended for localhost; TLS is not compiled in, so https targets are unsupported).
- Binary bodies only record their size and are not replayed byte-for-byte.
- The reserved path `/__hooklab` is not captured — handle separately if your webhook happens to use that prefix.

## License

MIT
