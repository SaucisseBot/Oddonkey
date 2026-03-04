# 🫏 OddOnkey

A dead-simple Rust wrapper around [Ollama](https://ollama.com). Auto-installs Ollama, auto-pulls models, and lets you prompt a local LLM in two lines of code.

```rust
let mut model = OddOnkey::new("mistral").await?;
let answer = model.prompt("What is the capital of France?").await?;
```

No config files. No Docker. No API keys. Just add the crate and go.

---

## Features

- **Zero setup** — automatically installs Ollama and pulls the requested model if needed.
- **Conversation history** — multi-turn chat with full context, out of the box.
- **Streaming** — token-by-token output via a standard `Stream` impl.
- **Embeddings** — single or batch embedding vectors in one call.
- **Generation options** — temperature, top-p, top-k, context size, repeat penalty, seed, etc.
- **Progress bar** *(opt-in)* — visual feedback during model download & server start.
- **Per-prompt report** *(opt-in)* — duration, estimated tokens, throughput, request/response sizes.
- **Hexagonal architecture** — swap the Ollama backend for any LLM by implementing one trait.

## Quick start

Add to your `Cargo.toml`:

```toml
[dependencies]
oddonkey = { git = "https://github.com/SaucisseBot/Oddonkey.git" }
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

## Optional features

Enable in `Cargo.toml`:

```toml
oddonkey = { git = "...", features = ["progress", "report"] }
```

| Feature    | Description                                              |
|------------|----------------------------------------------------------|
| `progress` | Shows an `indicatif` progress bar during model pull and a spinner while the server starts. |
| `report`   | Enables the `PromptReport` struct (also togglable at runtime via `.enable_report(true)`).   |
| `docker`   | Run Ollama inside a Docker container — zero local install outside Docker. Requires Docker on the host. |

## Usage

### Builder pattern

```rust
let mut model = OddOnkey::builder("mistral")
    .base_url("http://localhost:11434") // custom Ollama URL
    .progress(true)                     // show progress bar
    .report(true)                       // collect per-prompt stats
    .build()
    .await?;
```

### Docker mode (zero local install)

With the `docker` feature enabled, Ollama runs entirely inside a Docker container — nothing is installed on the host except Docker itself.

```toml
oddonkey = { git = "...", features = ["docker"] }
```

```rust
let mut model = OddOnkey::builder("mistral")
    .docker(true)           // run Ollama in Docker
    .docker_gpu(true)       // optional: GPU passthrough (requires NVIDIA Container Toolkit)
    .docker_port(11434)     // optional: custom host port
    .docker_cleanup(true)   // optional: remove container + data on drop (zero waste)
    .progress(true)
    .build()
    .await?;

// Use exactly like normal — same API
let answer = model.prompt("Hello!").await?;
// When `model` is dropped, the container and its volume are destroyed automatically.
```

The container (`oddonkey-ollama`) persists pulled models across restarts. You can also manage it directly via `DockerManager`:

```rust
use oddonkey::DockerManager;

let mgr = DockerManager::new().gpu(true);
mgr.stop()?;    // stop the container (models persist)
mgr.destroy()?; // stop + remove container and volume
```

### System pre-prompts

```rust
model.add_preprompt("You are a friendly pirate.");
// or replace all pre-prompts:
model.set_preprompt("You are a concise assistant.");
```

### Generation options

```rust
use oddonkey::GenerationOptions;

model.set_options(
    GenerationOptions::default()
        .temperature(0.3)
        .num_ctx(8192)
        .top_p(0.9)
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
model.push_assistant_message("Tell me a joke", &full);
```

### Embeddings

```rust
let vec = model.embed("Rust is awesome").await?;
let vecs = model.embed_batch(&["hello", "world"]).await?;
```

### Per-prompt report

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

## Architecture

OddOnkey uses a hexagonal (ports & adapters) architecture:

```
src/
├── lib.rs                  # re-exports
├── domain/                 # pure value objects (no I/O)
│   ├── error.rs            # OddOnkeyError
│   ├── message.rs          # ChatMessage
│   ├── options.rs          # GenerationOptions
│   └── report.rs           # PromptReport
├── ports/
│   └── llm_provider.rs     # LlmProvider trait
├── adapters/
│   ├── ollama/             # Ollama HTTP adapter
│   │   ├── client.rs       # LlmProvider implementation
│   │   ├── installer.rs    # auto-install & server start
│   │   ├── pull.rs         # model pull with progress
│   │   ├── stream.rs       # TokenStream
│   │   └── types.rs        # Ollama JSON DTOs
│   └── docker/             # Docker adapter (feature-gated)
│       └── manager.rs      # container lifecycle management
└── core/
    ├── oddonkey.rs          # OddOnkey struct (backend-agnostic)
    └── builder.rs           # OddOnkeyBuilder
```

To add a new backend, implement the `LlmProvider` trait — no changes to `core/` needed.

## Examples

```sh
cargo run --example chat                        # interactive pirate chat
cargo run --example stream                      # streaming token-by-token
cargo run --example embeddings                  # embeddings + cosine similarity
cargo run --example report --features report    # per-prompt stats
```

## License

MIT
