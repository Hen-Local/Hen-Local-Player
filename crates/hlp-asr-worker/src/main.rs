// Audio sanitizing and the minimum clip size are adapted from
// Hen-Local-Translator/node-hub/dora-qwen3-asr/src/main.rs.
use anyhow::{ensure, Context, Result};
use clap::Parser;
use hlp_core::{
    asr_protocol::{Hello, Loaded, Operation, Request, Response, Transcription},
    pcm::read_pcm,
};
use serde_json::{json, Value};
use std::{
    io::{self, BufRead, Write},
    panic::{catch_unwind, AssertUnwindSafe},
    path::{Path, PathBuf},
    time::Instant,
};

#[cfg(target_os = "macos")]
#[path = "backend/macos.rs"]
mod backend;
#[cfg(not(target_os = "macos"))]
#[path = "backend/windows.rs"]
mod backend;

#[derive(Parser)]
struct Args {
    #[arg(long,env="HLP_ASR_BACKEND",default_value=backend::NAME)]
    backend: String,
    #[arg(long, env = "HLP_ASR_MODEL_DIR")]
    model_dir: Option<PathBuf>,
}

const MIN_SAMPLES: u64 = 1600;
const SAMPLES_PER_MS: u64 = 16;

fn sanitize_audio_samples(samples: &mut [f32]) {
    for sample in samples {
        *sample = if sample.is_finite() {
            sample.clamp(-1.0, 1.0)
        } else {
            0.0
        };
    }
}

fn clip(path: &Path, start: u64, end: u64) -> Result<(Vec<f32>, u64)> {
    ensure!(end >= start, "endSample precedes startSample");
    let total = std::fs::metadata(path)?.len() / 2;
    let end = end.min(total);
    let start = start.min(total);
    let length = end - start;
    if length < MIN_SAMPLES {
        return Ok((Vec::new(), length / SAMPLES_PER_MS));
    }
    let mut samples = read_pcm(
        path,
        start.saturating_sub(100 * SAMPLES_PER_MS),
        end.saturating_add(150 * SAMPLES_PER_MS),
    )?;
    sanitize_audio_samples(&mut samples);
    Ok((samples, length / SAMPLES_PER_MS))
}

struct Worker {
    dir: PathBuf,
    engine: Option<backend::AsrEngine>,
}
impl Worker {
    fn handle(&mut self, op: Operation) -> Result<Value> {
        let start = Instant::now();
        match op {
            Operation::Hello => Ok(json!(Hello {
                protocol: 1,
                backend: backend::NAME.into(),
                platform: std::env::consts::OS.into(),
                can_force_language: backend::CAN_FORCE_LANGUAGE,
                reports_language: backend::REPORTS_LANGUAGE
            })),
            Operation::Load => {
                if self.engine.is_none() {
                    self.engine = Some(backend::AsrEngine::load(&self.dir)?);
                }
                Ok(json!(Loaded {
                    load_ms: start.elapsed().as_millis() as u64
                }))
            }
            Operation::Transcribe {
                pcm_path,
                start_sample,
                end_sample,
                language,
                max_tokens,
            } => {
                let engine = self.engine.as_mut().context("ASR model is not loaded")?;
                ensure!(
                    (1..=1024).contains(&max_tokens),
                    "maxTokens must be between 1 and 1024"
                );
                let (samples, audio_ms) = clip(&pcm_path, start_sample, end_sample)?;
                let raw = if samples.is_empty() {
                    String::new()
                } else {
                    engine.transcribe(&samples, language.as_deref(), max_tokens)?
                };
                Ok(json!(Transcription {
                    raw,
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    audio_ms
                }))
            }
            Operation::Unload | Operation::Shutdown => {
                drop(self.engine.take());
                backend::clear_cache();
                Ok(json!({}))
            }
        }
    }
}

fn run(input: impl BufRead, mut output: impl Write, worker: &mut Worker) -> Result<()> {
    for (count, line) in input.lines().enumerate() {
        let line = line?;
        let mut id = 0;
        let mut shutdown = false;
        let result = catch_unwind(AssertUnwindSafe(|| -> Result<Value> {
            let value: Value = serde_json::from_str(&line)?;
            id = value.get("id").and_then(Value::as_u64).unwrap_or(0);
            let req: Request = serde_json::from_value(value)?;
            shutdown = matches!(req.op, Operation::Shutdown);
            let result = worker.handle(req.op);
            if (count + 1) % 200 == 0 {
                backend::clear_cache();
            }
            result
        }))
        .unwrap_or_else(|_| {
            Err(anyhow::anyhow!(
                "ASR worker panicked while processing this request"
            ))
        });
        let response = match result {
            Ok(result) => Response {
                id,
                ok: true,
                result: Some(result),
                error: None,
            },
            Err(err) => Response {
                id,
                ok: false,
                result: None,
                error: Some(format!("{err:#}")),
            },
        };
        serde_json::to_writer(&mut output, &response)?;
        output.write_all(b"\n")?;
        output.flush()?;
        if shutdown {
            break;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::try_parse()?;
    ensure!(
        cfg!(any(target_os = "macos", windows)),
        "ASR is available on macOS and Windows"
    );
    ensure!(
        args.backend == backend::NAME,
        "Unsupported backend {}",
        args.backend
    );
    let dir = args.model_dir.unwrap_or_else(|| {
        let root = std::env::var_os("HLP_OMINIX_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".OminiX"));
        root.join("models").join(if cfg!(windows) {
            "sherpa-onnx-qwen3-asr-0.6B-int8-2026-03-25"
        } else {
            "qwen3-asr-1.7b"
        })
    });
    run(
        io::stdin().lock(),
        io::stdout().lock(),
        &mut Worker { dir, engine: None },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_request_does_not_break_protocol() {
        let mut output = Vec::new();
        let mut worker = Worker {
            dir: PathBuf::new(),
            engine: None,
        };
        run(
            &b"broken\n{\"id\":9,\"op\":\"unknown\"}\n{\"id\":10,\"op\":\"hello\"}\n"[..],
            &mut output,
            &mut worker,
        )
        .unwrap();
        let responses = output
            .lines()
            .map(|l| serde_json::from_str::<Response>(&l.unwrap()).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            responses.iter().map(|r| (r.id, r.ok)).collect::<Vec<_>>(),
            [(0, false), (9, false), (10, true)]
        );
    }
    #[test]
    fn handles_short_and_padded_pcm() {
        let f = tempfile::NamedTempFile::new().unwrap();
        f.as_file().set_len(16000 * 2).unwrap();
        assert!(clip(f.path(), 40, 48).unwrap().0.is_empty());
        assert_eq!(clip(f.path(), 8000, 9600).unwrap().0.len(), 5600);
        assert_eq!(clip(f.path(), 0, 16000).unwrap().0.len(), 16000);
        assert!(clip(f.path(), 2, 1).is_err());
        let mut samples = [f32::NAN, f32::INFINITY, -2., 2., 0.25];
        sanitize_audio_samples(&mut samples);
        assert_eq!(samples, [0., 0., -1., 1., 0.25]);
    }
}
