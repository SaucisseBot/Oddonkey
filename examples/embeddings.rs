use oddonkey::OddOnkey;

#[tokio::main]
async fn main() {
    let model_name = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "mistral".to_string());

    println!("→ Loading model '{model_name}'…");

    let model = OddOnkey::new(&model_name)
        .await
        .expect("failed to init OddOnkey");

    // --- Single embedding ---
    let text = "Rust is a systems programming language";
    let vec = model.embed(text).await.expect("embed failed");
    println!("Embedding for \"{text}\":");
    println!("  dimensions = {}", vec.len());
    println!("  first 5 values = {:?}", &vec[..5.min(vec.len())]);

    // --- Batch embeddings + similarity ---
    let sentences = [
        "The cat sat on the mat",
        "A kitten rested on the rug",
        "Quantum physics is fascinating",
    ];

    let vecs = model
        .embed_batch(&sentences.iter().map(|s| *s).collect::<Vec<_>>())
        .await
        .expect("embed_batch failed");

    println!("\nCosine similarities:");
    for i in 0..sentences.len() {
        for j in (i + 1)..sentences.len() {
            let sim = cosine_similarity(&vecs[i], &vecs[j]);
            println!(
                "  \"{}\"\n  \"{}\"  →  {sim:.4}\n",
                sentences[i], sentences[j]
            );
        }
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        0.0
    } else {
        dot / (mag_a * mag_b)
    }
}
