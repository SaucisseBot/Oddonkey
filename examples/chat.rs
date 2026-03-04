use oddonkey::OddOnkey;

#[tokio::main]
async fn main() {
    // Pick any Ollama model name – "mistral", "llama3", "phi3", "gemma", etc.
    let model_name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "mistral".to_string());

    println!("→ Loading model '{model_name}'…");

    let mut model = OddOnkey::new(&model_name)
        .await
        .expect("failed to init OddOnkey");

    model.add_preprompt("You are a friendly pirate. Answer everything in pirate speak.");

    loop {
        // Read user input
        eprint!("\nYou: ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input.is_empty() || input == "quit" || input == "exit" {
            break;
        }

        match model.prompt(input).await {
            Ok(reply) => println!("\nPirate: {reply}"),
            Err(e) => eprintln!("\n[error] {e}"),
        }
    }
}
