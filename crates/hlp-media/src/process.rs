use anyhow::{bail, Context, Result};
use std::{
    io::BufRead,
    path::Path,
    process::{ChildStderr, ChildStdout, Command, ExitStatus, Stdio},
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::Duration,
};
use tokio_util::sync::CancellationToken;

pub(crate) fn command(path: &Path) -> Command {
    let mut cmd = Command::new(path);
    cmd.stdin(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    cmd
}

pub(crate) fn kill_tree(pid: u32) {
    #[cfg(unix)]
    {
        // Children have a private process group whose ID is the child's PID.
        unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
    }
    #[cfg(windows)]
    {
        let _ = command(Path::new("taskkill"))
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .output();
    }
}

/// Keep cancellation responsive even while a decoder or a reader is blocked on a pipe.
pub(crate) fn run<T>(
    cmd: &mut Command,
    cancel: &CancellationToken,
    read: impl FnOnce(Option<ChildStdout>, ChildStderr) -> Result<T>,
) -> Result<(ExitStatus, T)> {
    if cancel.is_cancelled() {
        bail!("Cancelled");
    }
    let mut child = cmd
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Start {}", cmd.get_program().to_string_lossy()))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take().expect("piped stderr");
    thread::scope(|scope| {
        let (abort, control) = mpsc::channel::<()>();
        let monitor = scope.spawn(move || -> std::io::Result<ExitStatus> {
            loop {
                if let Some(status) = child.try_wait()? {
                    return Ok(status);
                }
                if cancel.is_cancelled()
                    || !matches!(
                        control.recv_timeout(Duration::from_millis(30)),
                        Err(RecvTimeoutError::Timeout)
                    )
                {
                    kill_tree(child.id());
                    let _ = child.kill();
                    return child.wait();
                }
            }
        });
        let result = read(stdout, stderr);
        if result.is_err() {
            let _ = abort.send(());
        }
        let status = monitor
            .join()
            .map_err(|_| anyhow::anyhow!("Child monitor stopped"))??;
        if cancel.is_cancelled() {
            bail!("Cancelled");
        }
        Ok((status, result?))
    })
}

pub(crate) fn error_tail(reader: impl BufRead) -> std::io::Result<String> {
    let mut last = String::new();
    for line in reader.lines() {
        let line = line?;
        if !line.trim().is_empty()
            && !line
                .split_once('=')
                .is_some_and(|(key, _)| key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'))
        {
            last = line;
        }
    }
    Ok(last)
}
