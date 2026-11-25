//! Adapted from Hen-Local-Translator/node-hub/dora-qwen35-translator/src/backend_llamacpp.rs.
use anyhow::{bail, ensure, Context, Result};
use hlp_core::cues::{align_translated_phrases, translation_plan, AlignedPhrase};
use hlp_core::prompts::{translation_prompt, MAX_TOKENS, REPEAT_PENALTY, TEMPERATURE};
use serde_json::{json, Value};
use std::{net::TcpListener, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    time::{sleep, Instant},
};

struct LlamaServer {
    child: Child,
    base_url: String,
    client: reqwest::Client,
}

#[derive(Debug, thiserror::Error)]
#[error("No usable translation was returned for this subtitle.")]
pub(super) struct UntranslatedLine;

impl LlamaServer {
    async fn spawn() -> Result<Self> {
        let port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
        let mut cmd = Command::new(hlp_models::llama_server_path());
        cmd.arg("-m")
            .arg(hlp_models::gguf_path())
            .args([
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "-c",
                "4096",
                "-ngl",
                if cfg!(windows) { "0" } else { "99" },
                "--jinja",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(0x08000000);
        let mut child = cmd.spawn().context("Start the local translation server")?;
        let mut lines = BufReader::new(
            child
                .stderr
                .take()
                .context("Missing llama-server diagnostics")?,
        )
        .lines();
        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                tracing::debug!(target:"llama_server","{line}")
            }
        });
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(90))
            .build()?;
        let mut server = Self {
            child,
            base_url: format!("http://127.0.0.1:{port}"),
            client,
        };
        let deadline = Instant::now() + Duration::from_secs(180);
        loop {
            if let Some(status) = server.child.try_wait()? {
                bail!("llama-server exited with {status}")
            }
            if server
                .client
                .get(format!("{}/health", server.base_url))
                .timeout(Duration::from_secs(1))
                .send()
                .await
                .is_ok_and(|r| r.status().is_success())
            {
                return Ok(server);
            }
            ensure!(
                Instant::now() < deadline,
                "Timed out loading the translation model"
            );
            sleep(Duration::from_millis(100)).await;
        }
    }
    async fn complete(&self, prompt: String) -> Result<String> {
        let response:Value=self.client.post(format!("{}/v1/chat/completions",self.base_url)).json(&json!({
            "messages":[{"role":"user","content":prompt}], "temperature":TEMPERATURE,"repeat_penalty":REPEAT_PENALTY,"max_tokens":MAX_TOKENS,"cache_prompt":true,"stream":false
        })).send().await?.error_for_status()?.json().await?;
        let message = &response["choices"][0]["message"];
        if message["content"].is_null() && message["reasoning_content"].is_string() {
            return Ok(String::new());
        }
        Ok(message["content"]
            .as_str()
            .context("Translation server returned no text")?
            .to_owned())
    }
}

#[derive(Default)]
pub struct Translator {
    server: Option<LlamaServer>,
}
impl Translator {
    pub fn new() -> Self {
        Self::default()
    }
    pub(super) async fn shutdown(&mut self) {
        if let Some(mut server) = self.server.take() {
            let _ = server.child.kill().await;
        }
    }
    #[cfg(test)]
    pub(super) fn process_id(&self) -> Option<u32> {
        self.server.as_ref()?.child.id()
    }
    pub async fn load(&mut self) -> Result<()> {
        if let Some(server) = &mut self.server {
            if server.child.try_wait()?.is_none() {
                return Ok(());
            }
        }
        self.server = Some(LlamaServer::spawn().await?);
        Ok(())
    }
    async fn request(&mut self, prompt: &str) -> Result<String> {
        let mut last = None;
        for attempt in 0..4 {
            let result = async {
                self.load().await?;
                self.server
                    .as_ref()
                    .unwrap()
                    .complete(prompt.to_owned())
                    .await
            }
            .await;
            match result {
                Ok(text) => return Ok(text),
                Err(e) => {
                    self.server.take();
                    tracing::warn!(
                        "Local translation server request failed (attempt {})",
                        attempt + 1
                    );
                    last = Some(e);
                    if attempt < 3 {
                        sleep(Duration::from_millis(150 << attempt)).await
                    }
                }
            }
        }
        Err(last.unwrap())
    }
    pub async fn translate(
        &mut self,
        source: Option<&str>,
        target: &str,
        text: &str,
        context: Option<&str>,
    ) -> Result<String> {
        for attempt in 0..2 {
            let prompt = if attempt == 0 {
                translation_prompt(source, target, text, context, false)
            } else {
                // The contextual template can repeatedly echo a fragment such as "I.".
                // Give the model the language and only the words that were spoken.
                let source = source.and_then(hlp_core::lang::lookup).map_or_else(
                    || "speech".to_owned(),
                    |l| format!("spoken {}", l.english_name),
                );
                let target = hlp_core::lang::lookup(target).map_or(target, |l| l.english_name);
                format!("Translate this fragment of {source} into {target}. It may be an incomplete sentence. Translate only the words spoken and return only the translation.\n\n{text}")
            };
            let raw = self.request(&prompt).await?;
            let clean = clean_output(&raw, text);
            if !clean.is_empty()
                && (clean != text.trim()
                    || source == Some(target)
                    || !text.chars().any(char::is_alphabetic))
            {
                return Ok(clean);
            }
        }
        Err(UntranslatedLine.into())
    }

    pub(crate) async fn translate_subtitle(
        &mut self,
        source: Option<&str>,
        target: &str,
        text: &str,
        context: Option<&str>,
    ) -> Result<(String, Vec<AlignedPhrase>)> {
        let whole = self.translate(source, target, text, context).await?;
        let Some(plan) = translation_plan(text, &whole) else {
            return Ok((whole, Vec::new()));
        };

        // The complete utterance supplies the meaning of each short extract,
        // including words spoken later in the same sentence.
        let background = match context {
            Some(previous) => format!("{previous}\n{text}"),
            None => text.to_owned(),
        };
        // Phrase translations are alignment hints only. The model can echo its
        // background, so never concatenate these outputs into the saved translation.
        let mut hints = Vec::with_capacity(plan.len());
        for (span, reusable) in plan {
            let hint = match reusable {
                Some(existing) => existing.to_owned(),
                None => match self
                    .translate(source, target, span.trim(), Some(&background))
                    .await
                {
                    Ok(part) => part,
                    Err(_) => return Ok((whole, Vec::new())),
                },
            };
            hints.push((span, hint));
        }
        let spans: Vec<_> = hints.iter().map(|(s, t)| (*s, t.as_str())).collect();
        let offsets = align_translated_phrases(text, &whole, &spans).unwrap_or_default();
        Ok((whole, offsets))
    }
}
fn clean_output(raw: &str, source: &str) -> String {
    let mut text = raw.trim();
    if "<think>".starts_with(text) {
        return String::new();
    }
    if let Some((_, answer)) = text.rsplit_once("</think>") {
        text = answer.trim();
    }
    if text.contains("<think>") {
        return String::new();
    }
    for label in ["Translation:", "译文：", "翻译："] {
        if let Some(s) = text.strip_prefix(label) {
            text = s.trim();
            break;
        }
    }
    for (open, close) in [('“', '”'), ('"', '"'), ('「', '」'), ('\'', '\'')] {
        if !source.starts_with(open) {
            if let Some(s) = text.strip_prefix(open).and_then(|s| s.strip_suffix(close)) {
                text = s.trim();
                break;
            }
        }
    }
    if [
        "The user wants me to translate",
        "Let me translate this",
        "用户提供的原文",
        "首先，我需要翻译",
    ]
    .iter()
    .any(|s| text.starts_with(s))
    {
        return String::new();
    }
    text.to_owned()
}

#[cfg(test)]
mod tests {
    use super::clean_output;

    #[test]
    fn reasoning_is_never_used_as_a_missing_translation() {
        assert_eq!(clean_output("internal reasoning</think>", "I."), "");
        assert_eq!(clean_output("<think>still thinking", "I."), "");
        assert_eq!(clean_output("<think>reasoning</think>我。", "I."), "我。");
    }
}
