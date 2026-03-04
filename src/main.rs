use oddonkey::{GenerationOptions, OddOnkey};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() {
    // 1. Build with ALL features: Docker, progress bar, report, cleanup on exit
    let builder = OddOnkey::builder("mistral");
    #[cfg(feature = "docker")]
    let builder = builder
        .docker(true) // run Ollama inside Docker (no local install)
        .docker_cleanup(true); // remove container + volume when done (zero waste)
    let mut model = builder
        .progress(true) // show spinner/progress bar during pull & start
        .report(true) // collect per-prompt stats
        .build()
        .await
        .expect("failed to initialise OddOnkey");

    // 2. Set generation options
    model.set_options(GenerationOptions::default().temperature(0.4).num_ctx(4096));

    // 3. System pre-prompt
    model.add_preprompt("You are a concise and helpful assistant. Answer in 2-4 sentences.");

    // 4. First prompt (full response)
    println!("─── Prompt 1 ───────────────────────────────\n");
    let answer = model
        .prompt("What is the capital of France?")
        .await
        .expect("prompt failed");

    println!("Assistant: {answer}");
    if let Some(r) = model.last_report() {
        println!("\n{r}\n");
    }

    // 5. Follow-up (history is kept automatically)
    println!("─── Prompt 2 ───────────────────────────────\n");
    let answer2 = model
        .prompt("And what about Germany?")
        .await
        .expect("prompt failed");

    println!("Assistant: {answer2}");
    if let Some(r) = model.last_report() {
        println!("\n{r}\n");
    }

    // 6. Streaming response
    println!("─── Prompt 3 (streaming) ───────────────────\n");
    let mut stream = model
        .prompt_stream("Tell me a fun fact about Rust the programming language.")
        .await
        .expect("prompt_stream failed");

    print!("Assistant: ");
    let mut full_reply = String::new();
    while let Some(tok) = stream.next().await {
        match tok {
            Ok(token) => {
                print!("{token}");
                full_reply.push_str(&token);
                use std::io::Write;
                std::io::stdout().flush().unwrap();
            }
            Err(e) => {
                eprintln!("\n[stream error] {e}");
                break;
            }
        }
    }
    println!("\n");

    // Save streamed exchange in history
    model.push_assistant_message(
        "Tell me a fun fact about Rust the programming language.",
        &full_reply,
    );

    // 7. Embeddings
    println!("─── Embeddings ─────────────────────────────\n");
    let vec = model.embed("Rust is awesome").await.expect("embed failed");
    println!("Embedding dimensions: {}", vec.len());
    println!("First 5 values: {:?}\n", &vec[..5.min(vec.len())]);

    // 8. Done — model is dropped here, Docker container is auto-destroyed
    println!("─── Cleanup ────────────────────────────────\n");
    println!("Dropping model — Docker container will be cleaned up automatically…");
    drop(model);
    println!("Done! Zero traces left on disk.");
}
