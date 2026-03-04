use oddonkey::{GenerationOptions, OddOnkey};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() {
    let model_name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "mistral".to_string());

    println!("→ Loading model '{model_name}'…");

    let mut model = OddOnkey::new(&model_name)
        .await
        .expect("failed to init OddOnkey");

    // Low temperature for more focused answers
    model.set_options(GenerationOptions::default().temperature(0.4));

    model.add_preprompt("You are a helpful assistant. Be concise.");

    println!("Type a message (or 'quit' to exit). Responses stream token-by-token.\n");

    loop {
        eprint!("You: ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input.is_empty() || input == "quit" || input == "exit" {
            break;
        }

        // Stream tokens as they arrive
        let mut stream = model
            .prompt_stream(input)
            .await
            .expect("prompt_stream failed");

        print!("\nAssistant: ");
        let mut full_reply = String::new();

        while let Some(token_result) = stream.next().await {
            match token_result {
                Ok(token) => {
                    print!("{token}");
                    full_reply.push_str(&token);
                    // Flush stdout so tokens appear immediately
                    use std::io::Write;
                    std::io::stdout().flush().unwrap();
                }
                Err(e) => {
                    eprintln!("\n[error] {e}");
                    break;
                }
            }
        }
        println!("\n");

        // Save the exchange in history so follow-ups have context
        model.push_assistant_message(input, &full_reply);
    }
}
