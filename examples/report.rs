use oddonkey::OddOnkey;

#[tokio::main]
async fn main() {
    // Build with report enabled (and progress if compiled with --features progress)
    let mut model = OddOnkey::builder("mistral")
        .report(true)
        .progress(true) // only effective with `cargo run --features progress`
        .build()
        .await
        .expect("failed to init OddOnkey");

    model.add_preprompt("You are a concise assistant. Answer in 1-3 sentences.");

    // --- First prompt ---
    let answer = model
        .prompt("Explain what Rust's borrow checker does.")
        .await
        .expect("prompt failed");

    println!("Assistant: {answer}\n");

    if let Some(report) = model.last_report() {
        println!("{report}\n");
    }

    // --- Second prompt (history grows → bigger request) ---
    let answer2 = model
        .prompt("How does it differ from a garbage collector?")
        .await
        .expect("prompt failed");

    println!("Assistant: {answer2}\n");

    if let Some(report) = model.last_report() {
        println!("{report}");
    }
}
