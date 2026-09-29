//! Producing variant and preview bytes: `ActiveStorage::Transformers::Vips` (image_processing
//! 1.14) and `ActiveStorage::Previewer::VideoPreviewer`.

use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tempfile::NamedTempFile;

use crate::content_types::{VIDEO_PREVIEW_ARGUMENTS, ffmpeg_path};
use crate::marshal::Value;
use crate::variation::Variation;
use crate::vips::Image;
use crate::{Error, Result};

/// How long ffmpeg may take to draw a preview frame before it's killed. Rails sets no limit, but
/// a crafted or hour-long video shouldn't hold a processing thread indefinitely.
pub const FFMPEG_TIMEOUT: Duration = Duration::from_secs(60);

/// `variation.transform(file)`: `ImageProcessing::Vips.source(file).loader(page: 0)
/// .convert(format).apply(operations).call`, saved to a tempfile named for the format.
pub fn transform(input: &Path, variation: &Variation) -> Result<NamedTempFile> {
    let format = variation.format()?;
    let operations = operations(variation)?;

    let mut image = Image::load_for_processing(input)?;
    for (width, height) in operations {
        image = image.resize_to_limit(width, height)?;
    }

    let output = tempfile::Builder::new().prefix("image_processing").suffix(&format!(".{format}")).tempfile()?;
    image.write_to_file(output.path())?;
    Ok(output)
}

/// `ImageProcessingTransformer#operations`: every transformation except `format`, skipping blank
/// arguments. Only `resize_to_limit` is implemented: it's the only one Campfire defines.
fn operations(variation: &Variation) -> Result<Vec<(Option<i32>, Option<i32>)>> {
    let mut operations = Vec::new();
    for (name, argument) in variation.transformations() {
        match (name.as_str(), argument) {
            ("format", _) => {}
            ("combine_options", _) => {
                return Err(Error::InvalidVariation("combine_options is not supported".into()));
            }
            (_, argument) if blank(argument) => {}
            ("resize_to_limit", Value::Array(args)) if args.len() == 2 => {
                let dimension = |v: Option<&Value>| match v {
                    None | Some(Value::Nil) => Ok(None),
                    Some(Value::Int(n)) => Ok(Some(*n as i32)),
                    Some(other) => Err(Error::InvalidVariation(format!("resize_to_limit argument {other:?}"))),
                };
                let (width, height) = (dimension(args.first())?, dimension(args.get(1))?);
                if width.is_none() && height.is_none() {
                    return Err(Error::InvalidVariation("either width or height must be specified".into()));
                }
                operations.push((width, height));
            }
            (name, argument) => {
                return Err(Error::InvalidVariation(format!("unsupported transformation {name}: {argument:?}")));
            }
        }
    }
    Ok(operations)
}

/// `Object#present?` negated, for the values a transformation can hold.
fn blank(value: &Value) -> bool {
    match value {
        Value::Nil | Value::Bool(false) => true,
        Value::Str(s) => s.trim().is_empty(),
        Value::Array(items) => items.is_empty(),
        Value::Hash(entries) => entries.is_empty(),
        _ => false,
    }
}

/// `VideoPreviewer.accept?`: `system(ffmpeg, "-version")`, memoized.
pub fn ffmpeg_exists() -> bool {
    static EXISTS: OnceLock<bool> = OnceLock::new();
    *EXISTS.get_or_init(|| {
        Command::new(ffmpeg_path())
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    })
}

/// `draw_relevant_frame_from`: `ffmpeg -i <input> <video_preview_arguments> -`, capturing stdout.
pub fn video_preview(input: &Path) -> Result<Vec<u8>> {
    let mut command = Command::new(ffmpeg_path());
    command.arg("-i").arg(input).args(VIDEO_PREVIEW_ARGUMENTS).arg("-").stderr(Stdio::piped());
    let output = output_within(&mut command, FFMPEG_TIMEOUT).map_err(|error| match error.kind() {
        std::io::ErrorKind::TimedOut => Error::Preview(format!("{} {error}", ffmpeg_path())),
        _ => error.into(),
    })?;
    if !output.status.success() {
        return Err(Error::Preview(format!(
            "{} failed (status {}): {}",
            ffmpeg_path(),
            output.status.code().map_or("nil".into(), |c| c.to_string()),
            String::from_utf8_lossy(&output.stderr).trim_end()
        )));
    }
    Ok(output.stdout)
}

/// `command.output()`, except that the child is killed (and reaped) once `timeout` passes, which
/// is an `ErrorKind::TimedOut` error. Stdin is closed and stdout captured; stderr is captured
/// only when the caller pipes it.
pub fn output_within(command: &mut Command, timeout: Duration) -> std::io::Result<Output> {
    let mut child = command.stdin(Stdio::null()).stdout(Stdio::piped()).spawn()?;
    // Drain the pipes while waiting, so a chatty child can't stall on a full pipe.
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let Some(status) = wait_until(&mut child, Instant::now() + timeout)? else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, format!("timed out after {:?}", timeout)));
    };
    Ok(Output { status, stdout: stdout.join().unwrap_or_default(), stderr: stderr.join().unwrap_or_default() })
}

fn drain(pipe: Option<impl Read + Send + 'static>) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    })
}

/// The child's exit status, or `None` while it's still running at `deadline`.
fn wait_until(child: &mut Child, deadline: Instant) -> std::io::Result<Option<ExitStatus>> {
    let mut pause = Duration::from_millis(1);
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        let now = Instant::now();
        if now >= deadline {
            return Ok(None);
        }
        std::thread::sleep(pause.min(deadline - now));
        pause = (pause * 2).min(Duration::from_millis(50));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_within_captures_a_quick_child() {
        let mut command = Command::new("sh");
        command.args(["-c", "echo out; echo err >&2"]).stderr(Stdio::piped());
        let output = output_within(&mut command, Duration::from_secs(10)).unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"out\n");
        assert_eq!(output.stderr, b"err\n");
    }

    #[test]
    fn output_within_kills_a_child_that_overruns() {
        let started = Instant::now();
        let pid_file = tempfile::NamedTempFile::new().unwrap();
        let mut command = Command::new("sh");
        command.arg("-c").arg(format!("echo $$ > {}; exec sleep 30", pid_file.path().display()));
        let error = output_within(&mut command, Duration::from_millis(300)).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(5), "took {:?}", started.elapsed());
        let pid = std::fs::read_to_string(pid_file.path()).unwrap();
        assert!(!Path::new(&format!("/proc/{}", pid.trim())).exists(), "the child is still running");
    }
}
