use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tempfile::NamedTempFile;

#[derive(Debug)]
pub struct TraceResult {
    pub exit_code: i32,
    pub wall_time: Duration,
    pub trace_contents: String,
    pub timed_out: bool,
    pub kept_trace_path: Option<PathBuf>,
}

pub fn run_trace(
    program: &str,
    program_args: &[String],
    timeout_secs: u64,
    keep_trace: bool,
) -> Result<TraceResult, String> {
    let temp_trace =
        NamedTempFile::new().map_err(|err| format!("Could not create temp trace file: {}", err))?;

    let trace_path = temp_trace.path().to_path_buf();
    let start = Instant::now();

    let mut child = Command::new("strace")
        .args(["-f", "-tt", "-T", "-o"])
        .arg(&trace_path)
        .arg(program)
        .args(program_args)
        .spawn()
        .map_err(|err| format!("Could not start strace. Is it installed?\nOriginal error: {}", err))?;

    let timeout = Duration::from_secs(timeout_secs);
    let mut timed_out = false;

    let exit_code = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                break status.code().unwrap_or(-1);
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    timed_out = true;

                    child
                        .kill()
                        .map_err(|err| format!("Timed out, but failed to stop strace: {}", err))?;

                    let status = child
                        .wait()
                        .map_err(|err| format!("Timed out, and failed while waiting for strace: {}", err))?;

                    break status.code().unwrap_or(-1);
                }

                sleep(Duration::from_millis(50));
            }
            Err(err) => {
                return Err(format!("Failed while waiting on strace: {}", err));
            }
        }
    };

    let wall_time = start.elapsed();

    let trace_contents = fs::read_to_string(&trace_path)
        .map_err(|err| format!("Could not read trace file {}: {}", trace_path.display(), err))?;

    let kept_trace_path = if keep_trace {
        let keep_path = make_kept_trace_path();
        temp_trace
            .persist(&keep_path)
            .map_err(|err| format!("Could not keep trace file at {}: {}", keep_path.display(), err.error))?;
        Some(keep_path)
    } else {
        None
    };

    Ok(TraceResult {
        exit_code,
        wall_time,
        trace_contents,
        timed_out,
        kept_trace_path,
    })
}

fn make_kept_trace_path() -> PathBuf {
    let mut path = std::env::temp_dir();

    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    path.push(format!("why-{}-{}.trace", std::process::id(), timestamp_ms));
    path
}