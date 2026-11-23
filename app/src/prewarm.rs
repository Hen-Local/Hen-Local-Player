use hlp_core::settings::Settings;
use hlp_pipeline::SharedEngines;

pub async fn run(settings: Settings, engines: SharedEngines) {
    let ready = tokio::task::spawn_blocking(hlp_models::status)
        .await
        .unwrap_or_default();
    let has = |id: &str| ready.iter().any(|c| c.id == id && c.ready);
    let id = match settings.asr_model.as_str() {
        "qwen3-asr-0.6b" => "asr-qwen3-0.6b-mlx",
        "sherpa-qwen3-asr-0.6b" => "asr-qwen3-0.6b-sherpa",
        _ => "asr-qwen3-1.7b-mlx",
    };
    if has(id) {
        if let Err(e) = engines.asr.lock().await.load().await {
            tracing::warn!(error=%e,"Recognition warm-up failed");
        }
    }
    if has("mt-hy-mt2-1.8b") && has("llama-runtime-b11429") && settings.target_language != "none" {
        if let Err(e) = engines
            .translator
            .lock()
            .await
            .translate(Some("en"), "zh", "Hello.", None)
            .await
        {
            tracing::warn!(error=%e,"Translation warm-up failed");
        }
    }
}
