use std::time::Duration;

use crate::adapters::ollama::types::ModelList;
use crate::domain::error::OddOnkeyError;

/// Make sure the requested model has been pulled. Pull it if missing.
pub async fn ensure_model_available(
    client: &reqwest::Client,
    base_url: &str,
    model_name: &str,
    show_progress: bool,
) -> Result<(), OddOnkeyError> {
    let resp = client.get(format!("{base_url}/api/tags")).send().await?;

    let list: ModelList = resp
        .json()
        .await
        .unwrap_or(ModelList { models: Vec::new() });

    let needed = model_name.split(':').next().unwrap_or(model_name);
    let already_present = list.models.iter().any(|m| {
        let local = m.name.split(':').next().unwrap_or(&m.name);
        local == needed
    });

    if already_present {
        return Ok(());
    }

    eprintln!("[oddonkey] pulling model '{model_name}' – this may take a while…");

    #[cfg(feature = "progress")]
    if show_progress {
        return pull_with_progress(client, base_url, model_name).await;
    }

    #[cfg(not(feature = "progress"))]
    let _ = show_progress;

    let resp = client
        .post(format!("{base_url}/api/pull"))
        .json(&serde_json::json!({ "name": model_name, "stream": false }))
        .timeout(Duration::from_secs(3600))
        .send()
        .await?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(OddOnkeyError::ModelPullFailed(format!(
            "HTTP {status}: {text}"
        )));
    }

    eprintln!("[oddonkey] model '{model_name}' ready.");
    Ok(())
}

/// Pull a model while streaming JSON progress events into an indicatif bar.
#[cfg(feature = "progress")]
async fn pull_with_progress(
    client: &reqwest::Client,
    base_url: &str,
    model_name: &str,
) -> Result<(), OddOnkeyError> {
    use std::time::Duration;
    use tokio_stream::StreamExt;

    let resp = client
        .post(format!("{base_url}/api/pull"))
        .json(&serde_json::json!({ "name": model_name, "stream": true }))
        .timeout(Duration::from_secs(3600))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(OddOnkeyError::ModelPullFailed(format!(
            "HTTP {status}: {text}"
        )));
    }

    let bar = indicatif::ProgressBar::new(0);
    bar.set_style(
        indicatif::ProgressStyle::with_template(
            "{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}",
        )
        .unwrap()
        .progress_chars("█▓▒░  "),
    );
    bar.set_message(format!("pulling {model_name}"));
    bar.enable_steady_tick(Duration::from_millis(200));

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(OddOnkeyError::Http)?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find('\n') {
            let line = buffer[..pos].trim().to_string();
            buffer = buffer[pos + 1..].to_string();

            if line.is_empty() {
                continue;
            }

            if let Ok(obj) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(total) = obj.get("total").and_then(|v| v.as_u64()) {
                    bar.set_length(total);
                }
                if let Some(completed) = obj.get("completed").and_then(|v| v.as_u64()) {
                    bar.set_position(completed);
                }
                if let Some(status) = obj.get("status").and_then(|v| v.as_str()) {
                    bar.set_message(status.to_string());
                }
            }
        }
    }

    bar.finish_with_message(format!("{model_name} ready ✓"));
    Ok(())
}
