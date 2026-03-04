use oddonkey::OddOnkey;

#[tokio::main]
async fn main() {
    // 1. Create a model handle – installs ollama & pulls the model if needed
    let mut model = OddOnkey::builder("mistral")
        .report(true)
        .build()
        .await
        .expect("failed to initialise OddOnkey");

    // 2. (Optional) set a system pre-prompt
    model.add_preprompt("You are a concise and helpful assistant. Answer in 10-30 sentences.");

    // 3. Prompt!
    let answer = model
        .prompt("What is the capital of France?")
        .await
        .expect("prompt failed");

    println!("Assistant: {answer}");
    if let Some(r) = model.last_report() {
        println!("{r}\n");
    }

    // Follow-up (history is kept automatically)
    let answer2 = model
        .prompt("And what about Germany?")
        .await
        .expect("prompt failed");

    println!("Assistant: {answer2}");
    if let Some(r)  = model.last_report() {
        println!("{r}");
    }

    
}
