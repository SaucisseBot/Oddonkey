# 🫏 OddOnkey

[![Crates.io](https://img.shields.io/crates/v/oddonkey.svg)](https://crates.io/crates/oddonkey)
[![docs.rs](https://docs.rs/oddonkey/badge.svg)](https://docs.rs/oddonkey)
[![MIT licensed](https://img.shields.io/crates/l/oddonkey.svg)](./LICENSE)

A dead-simple Rust wrapper around [Ollama](https://ollama.com).
Auto-installs Ollama, auto-pulls models, and lets you prompt a local LLM in **two lines of code**.

```rust
let mut model = OddOnkey::new("mistral").await?;
let answer = model.prompt("What is the capital of France?").await?;
```

No config files. No API keys. Just add the crate and go.

---

## Table of Contents

- [Features](#features)
- [Quick Start](#quick-start)
- [Optional Features](#optional-features)
- [Usage](#usage)
  - [Builder Pattern](#builder-pattern)
  - [Docker Mode](#docker-mode-zero-local-install)
  - [System Pre-prompts](#system-pre-prompts)
  - [Generation Options](#generation-options)
  - [Streaming](#streaming)
  - [Embeddings](#embeddings)
  - [Per-prompt Report](#per-prompt-report)
- [Architecture](#architecture)
- [Examples](#examples)
- [Minimum Supported Rust Version](#minimum-supported-rust-version)
- [Contributing](#contributing)
- [License](#license)

---

## Features

| | |
|---|---|
| **Zero setup** | Automatically installs Ollama and pulls the requested model if needed. |
| **Conversation history** | Multi-turn chat with full context, out of the box. |
| **Streaming** | Token-by-token output via a standard `Stream` implementation. |
| **Embeddings** | Single or batch embedding vectors in one call. |
| **Generation options** | Temperature, top-p, top-k, context size, repeat penalty, seed, and more. |
| **Progress bar** *(opt-in)* | Visual feedback during model download and server start. |
| **Per-prompt report** *(opt-in)* | Duration, estimated tokens, throughput, request/response sizes. |
| **Docker mode** *(opt-in)* | Run Ollama in a container — zero local install outside Docker. |
| **Hexagonal architecture** | Swap the Ollama backend for any LLM by implementing one trait. |

---

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
oddonkey = "0.2"
tokio = { version = "1", features = ["full"] }
```

Then:

```rust
use oddonkey::OddOnkey;

#[tokio::main]
async fn main() {
    let mut model = OddOnkey::new("mistral").await.unwrap();
    model.add_preprompt("You are a helpful assistant.");
    let answer = model.prompt("What is 2+2?").await.unwrap();
    println!("{answer}");
}
```

That's it — Ollama is installed and the model is pulled automatically on first run.

---

## Optional Features

Enable in `Cargo.toml`:

```toml
oddonkey = { version = "0.2", features = ["progress", "report"] }
```

| Feature    | Description |
|------------|-------------|
| `progress` | Shows an [`indicatif`](https://crates.io/crates/indicatif) progress bar during model pull and a spinner while the server starts. |
| `report`   | Enables the `PromptReport` struct (also togglable at runtime via `.enable_report(true)`). |
| `docker`   | Run Ollama inside a Docker container — zero local install outside Docker. Requires Docker on the host. |

---

## Usage

### Builder Pattern

For fine-grained control, use the builder:

```rust
let mut model = OddOnkey::builder("mistral")
    .base_url("http://localhost:11434") // custom Ollama URL
    .progress(true)                     // show progress bar
    .report(true)                       // collect per-prompt stats
    .build()
    .await?;
```

### Docker Mode (zero local install)

With the `docker` feature enabled, Ollama runs entirely inside a Docker container — nothing is installed on the host except Docker itself.

```toml
oddonkey = { version = "0.2", features = ["docker"] }
```

```rust
let mut model = OddOnkey::builder("mistral")
    .docker(true)           // run Ollama in Docker
    .docker_gpu(true)       // optional: GPU passthrough (NVIDIA Container Toolkit)
    .docker_port(11434)     // optional: custom host port
    .docker_cleanup(true)   // optional: remove container + data on drop
    .progress(true)
    .build()
    .await?;

// Same API as always
let answer = model.prompt("Hello!").await?;
// When `model` is dropped, the container and its volume are destroyed automatically.
```

The container (`oddonkey-ollama`) persists pulled models across restarts by default. Enable `docker_cleanup(true)` for zero-trace disposable runs.

You can also manage the container directly:

```rust
use oddonkey::DockerManager;

let mgr = DockerManager::new().gpu(true);
mgr.stop()?;    // stop the container (models persist)
mgr.destroy()?; // stop + remove container and volume
```

### System Pre-prompts

```rust
model.add_preprompt("You are a friendly pirate.");
// or replace all pre-prompts:
model.set_preprompt("You are a concise assistant.");
```

### Generation Options

```rust
use oddonkey::GenerationOptions;

model.set_options(
    GenerationOptions::default()
        .temperature(0.3)
        .num_ctx(8192)
        .top_p(0.9)
        .top_k(40)
        .repeat_penalty(1.1)
        .seed(42)
);
```

### Streaming

```rust
use tokio_stream::StreamExt;

let mut stream = model.prompt_stream("Tell me a joke").await?;
let mut full = String::new();
while let Some(tok) = stream.next().await {
    let tok = tok?;
    print!("{tok}");
    full.push_str(&tok);
}
// Save the exchange in history for follow-up context
model.push_assistant_message("Tell me a joke", &full);
```

### Embeddings

```rust
// Single text
let vec = model.embed("Rust is awesome").await?;

// Batch
let vecs = model.embed_batch(&["hello", "world"]).await?;
```

### Per-prompt Report

Enable with `.report(true)` on the builder or `.enable_report(true)` at runtime:

```rust
let answer = model.prompt("Explain borrow checking.").await?;
if let Some(report) = model.last_report() {
    println!("{report}");
}
```

Output:

```
── report ──────────────────────────────────
model           : mistral
duration        : 1423 ms
prompt tokens   : ~12 (est.)
completion tkns : ~87 (est.)
tokens/sec      : 61.1
request size    : 245 bytes
response size   : 534 bytes
────────────────────────────────────────────
```

---

## Architecture

OddOnkey uses a **hexagonal (ports & adapters)** architecture. The core logic knows nothing about HTTP or Docker — it depends only on the `LlmProvider` trait.

```
src/
├── lib.rs                  # public re-exports
├── core/
│   ├── oddonkey.rs          # OddOnkey struct (backend-agnostic)
│   └── builder.rs           # OddOnkeyBuilder
├── domain/                 # pure value objects (no I/O)
│   ├── error.rs             # OddOnkeyError
│   ├── message.rs           # ChatMessage
│   ├── options.rs           # GenerationOptions
│   └── report.rs            # PromptReport
├── ports/
│   └── llm_provider.rs      # LlmProvider trait
└── adapters/
    ├── ollama/              # Ollama HTTP adapter
    │   ├── client.rs         # LlmProvider implementation
    │   ├── installer.rs      # auto-install & server start
    │   ├── pull.rs           # model pull with optional progress
    │   ├── stream.rs         # TokenStream
    │   └── types.rs          # Ollama JSON DTOs
    └── docker/              # Docker adapter (feature-gated)
        └── manager.rs        # container lifecycle management
```

To add a new backend (e.g. llama.cpp, vLLM, a remote API), implement the `LlmProvider` trait — no changes to `core/` required.

---

## Examples

Run the bundled examples:

```sh
# Interactive pirate chat
cargo run --example chat

# Streaming token-by-token
cargo run --example stream

# Embeddings + cosine similarity
cargo run --example embeddings

# Per-prompt timing report
cargo run --example report --features report

# Use a specific model
cargo run --example chat -- llama3
```

---

## Minimum Supported Rust Version

OddOnkey targets **Rust 1.75+** (edition 2021).

---

## Contributing

Contributions are welcome! Please open an issue or submit a pull request on [GitHub](https://github.com/SaucisseBot/Oddonkey).

1. Fork the repo
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Commit your changes (`git commit -m "feat: add my feature"`)
4. Push to the branch (`git push origin feat/my-feature`)
5. Open a Pull Request

---

## License

[MIT](./LICENSE)
