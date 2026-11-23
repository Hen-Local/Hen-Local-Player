//! A single owner serializes JSONL exchanges, even if a caller cancels its future.
use anyhow::{anyhow, Context, Result};
use hlp_core::asr_protocol::{Hello, Loaded, Operation, Request, Response, Transcription};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::{mpsc, oneshot},
    time::{sleep, timeout, Instant},
};

const TIMEOUT: Duration = Duration::from_secs(120);
const RESTART_WINDOW: Duration = Duration::from_secs(600);
const MAX_RESTARTS: usize = 3;

fn worker_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("HLP_ASR_WORKER") {
        return Ok(path.into());
    }
    let exe = std::env::current_exe()?;
    Ok(exe.with_file_name(format!("hlp-asr-worker{}", std::env::consts::EXE_SUFFIX)))
}

#[derive(Debug, thiserror::Error)]
enum Failure {
    #[error("ASR worker communication: {0:#}")]
    Transport(#[from] anyhow::Error),
    #[error("ASR worker: {0}")]
    Request(String),
}

struct Peer {
    process: Child,
    input: ChildStdin,
    lines: Lines<BufReader<ChildStdout>>,
    next_id: u64,
    hello: Option<Hello>,
    loaded: bool,
}

impl Peer {
    fn spawn(dir: &Path) -> Result<Self> {
        let path = worker_path()?;
        let mut cmd = Command::new(&path);
        cmd.arg("--model-dir")
            .arg(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(windows)]
        cmd.creation_flags(0x08000000);
        let mut process = cmd
            .spawn()
            .with_context(|| format!("Start {}", path.display()))?;
        let input = process.stdin.take().context("worker has no stdin")?;
        let lines = BufReader::new(process.stdout.take().context("worker has no stdout")?).lines();
        let mut errors =
            BufReader::new(process.stderr.take().context("worker has no stderr")?).lines();
        tokio::spawn(async move {
            while let Ok(Some(line)) = errors.next_line().await {
                tracing::debug!(target:"hlp_asr_worker","{line}");
            }
        });
        Ok(Self {
            process,
            input,
            lines,
            next_id: 1,
            hello: None,
            loaded: false,
        })
    }

    async fn exchange<T: DeserializeOwned>(
        &mut self,
        op: Operation,
    ) -> std::result::Result<T, Failure> {
        let id = self.next_id;
        self.next_id += 1;
        let mut encoded =
            serde_json::to_vec(&Request { id, op }).context("Serialize worker request")?;
        encoded.push(b'\n');
        let response: Response = timeout(TIMEOUT, async {
            self.input.write_all(&encoded).await?;
            self.input.flush().await?;
            let line = self
                .lines
                .next_line()
                .await?
                .context("worker closed stdout")?;
            let response: Response = serde_json::from_str(&line).context("Invalid worker JSON")?;
            anyhow::ensure!(
                response.id == id,
                "worker response ID {} does not match {}",
                response.id,
                id
            );
            Ok::<_, anyhow::Error>(response)
        })
        .await
        .context("worker request timed out")??;
        if !response.ok {
            return Err(Failure::Request(
                response.error.unwrap_or_else(|| "unspecified error".into()),
            ));
        }
        Ok(
            serde_json::from_value(response.result.context("worker omitted result")?)
                .context("Invalid worker result")?,
        )
    }

    async fn ready(&mut self) -> std::result::Result<Hello, Failure> {
        if self.hello.is_none() {
            let hello: Hello = self.exchange(Operation::Hello).await?;
            if hello.protocol != 1 {
                return Err(Failure::Request(format!(
                    "unsupported protocol {}",
                    hello.protocol
                )));
            }
            self.hello = Some(hello);
        }
        if !self.loaded {
            let _: Loaded = self.exchange(Operation::Load).await?;
            self.loaded = true;
        }
        Ok(self.hello.as_ref().unwrap().clone())
    }

    async fn handle(&mut self, op: &Operation) -> std::result::Result<Value, Failure> {
        match op {
            Operation::Load => Ok(json!(self.ready().await?)),
            Operation::Transcribe { .. } => {
                self.ready().await?;
                self.exchange(op.clone()).await
            }
            Operation::Unload => {
                let value = self.exchange(op.clone()).await?;
                self.loaded = false;
                Ok(value)
            }
            _ => self.exchange(op.clone()).await,
        }
    }
}

struct Supervisor {
    dir: PathBuf,
    peer: Option<Peer>,
    restarts: VecDeque<Instant>,
    failed: bool,
}

impl Supervisor {
    fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            peer: None,
            restarts: VecDeque::new(),
            failed: false,
        }
    }

    async fn stop(&mut self) {
        if let Some(mut peer) = self.peer.take() {
            // kill also waits/reaps when the child already exited.
            let _ = peer.process.kill().await;
        }
    }

    async fn run(&mut self, op: &Operation) -> Result<Value> {
        if matches!(op, Operation::Shutdown | Operation::Unload) && self.peer.is_none() {
            return Ok(json!({}));
        }
        loop {
            if self.peer.is_none() {
                if self.failed {
                    let now = Instant::now();
                    self.restarts
                        .retain(|t| now.duration_since(*t) < RESTART_WINDOW);
                    if self.restarts.len() >= MAX_RESTARTS {
                        anyhow::bail!("AsrUnavailable: restart limit reached (3 per 10 minutes)");
                    }
                    sleep(Duration::from_millis(100 << self.restarts.len())).await;
                    self.restarts.push_back(Instant::now());
                }
                self.peer =
                    Some(Peer::spawn(&self.dir).map_err(|e| anyhow!("AsrUnavailable: {e:#}"))?);
            }
            let peer = self.peer.as_mut().unwrap();
            let result = match peer.process.try_wait()? {
                Some(status) => Err(Failure::Transport(anyhow!("worker exited with {status}"))),
                None => peer.handle(op).await,
            };
            match result {
                Ok(value) => {
                    self.failed = false;
                    if matches!(op, Operation::Shutdown) {
                        self.stop().await;
                    }
                    return Ok(value);
                }
                Err(err @ Failure::Request(_)) => return Err(err.into()),
                Err(Failure::Transport(err)) => {
                    tracing::warn!("ASR worker lost: {err:#}");
                    self.stop().await;
                    self.failed = true;
                    if matches!(op, Operation::Shutdown | Operation::Unload) {
                        return Ok(json!({}));
                    }
                }
            }
        }
    }
}

struct Job {
    op: Operation,
    reply: oneshot::Sender<Result<Value>>,
}

pub struct AsrClient {
    dir: PathBuf,
    jobs: Option<mpsc::Sender<Job>>,
}

impl AsrClient {
    pub fn new(model_dir: PathBuf) -> Self {
        Self {
            dir: model_dir,
            jobs: None,
        }
    }

    fn jobs(&mut self) -> Result<&mpsc::Sender<Job>> {
        if self.jobs.is_none() {
            let (jobs, mut rx) = mpsc::channel::<Job>(1);
            let mut supervisor = Supervisor::new(self.dir.clone());
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            std::thread::Builder::new()
                .name("hlp-asr-client".into())
                .spawn(move || {
                    runtime.block_on(async move {
                        while let Some(job) = rx.recv().await {
                            let result = supervisor.run(&job.op).await;
                            let _ = job.reply.send(result);
                        }
                        supervisor.stop().await;
                    })
                })?;
            self.jobs = Some(jobs);
        }
        Ok(self.jobs.as_ref().unwrap())
    }

    async fn request<T: DeserializeOwned>(&mut self, op: Operation) -> Result<T> {
        let (reply, response) = oneshot::channel();
        self.jobs()?
            .send(Job { op, reply })
            .await
            .map_err(|_| anyhow!("ASR supervisor stopped"))?;
        Ok(serde_json::from_value(
            response.await.context("ASR supervisor stopped")??,
        )?)
    }

    pub async fn load(&mut self) -> Result<Hello> {
        self.request(Operation::Load).await
    }

    pub async fn transcribe(
        &mut self,
        pcm: &Path,
        start_sample: u64,
        end_sample: u64,
        language: Option<&str>,
        max_tokens: u32,
    ) -> Result<Transcription> {
        let duration_s = end_sample.saturating_sub(start_sample).div_ceil(16000);
        let budget = duration_s.saturating_mul(20).saturating_add(48).min(1024) as u32;
        self.request(Operation::Transcribe {
            pcm_path: pcm.to_owned(),
            start_sample,
            end_sample,
            language: language.map(str::to_owned),
            max_tokens: max_tokens.clamp(1, budget),
        })
        .await
    }

    pub async fn unload(&mut self) -> Result<()> {
        if self.jobs.is_some() {
            self.request::<Value>(Operation::Unload).await?;
        }
        Ok(())
    }
    pub async fn shutdown(&mut self) {
        if self.jobs.is_some() {
            let _ = self.request::<Value>(Operation::Shutdown).await;
            self.jobs = None;
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "requires the MLX worker, installed model and generated Japanese fixture"]
    async fn real_worker_restarts_after_kill() -> Result<()> {
        let model = std::env::var_os("HLP_ASR_MODEL_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap()
                    .join(".OminiX/models/qwen3-asr-1.7b")
            });
        let pcm = tempfile::NamedTempFile::new()?;
        let ffmpeg = std::env::var_os("HLP_FFMPEG").unwrap_or("ffmpeg".into());
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/fixture_ja.mp4");
        let status = Command::new(ffmpeg)
            .args(["-nostdin", "-loglevel", "error", "-y", "-i"])
            .arg(fixture)
            .args(["-vn", "-ac", "1", "-ar", "16000", "-f", "s16le"])
            .arg(pcm.path())
            .status()
            .await?;
        anyhow::ensure!(status.success(), "fixture extraction failed");
        let mut supervisor = Supervisor::new(model);
        supervisor.run(&Operation::Load).await?;
        let old = supervisor.peer.as_ref().unwrap().process.id().unwrap();
        let killed = Command::new("kill")
            .args(["-KILL", &old.to_string()])
            .status()
            .await?;
        anyhow::ensure!(killed.success(), "worker kill failed");
        let op = Operation::Transcribe {
            pcm_path: pcm.path().into(),
            start_sample: 8000,
            end_sample: 42000,
            language: None,
            max_tokens: 108,
        };
        let result: Transcription = serde_json::from_value(supervisor.run(&op).await?)?;
        assert!(
            result.raw.starts_with("language Japanese<asr_text>"),
            "{:?}",
            result.raw
        );
        let new = supervisor.peer.as_ref().unwrap().process.id().unwrap();
        assert_ne!(new, old);
        assert_eq!(supervisor.restarts.len(), 1);
        supervisor.run(&Operation::Shutdown).await?;
        Ok(())
    }
}
