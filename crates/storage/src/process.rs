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

/// `VideoPreviewer.accept?`: `system(ffmpeg, "-version")`, memoized once the binary has run
/// or is missing. A timed-out probe is not remembered, so a later call can try again.
pub fn ffmpeg_exists() -> bool {
    static EXISTS: OnceLock<bool> = OnceLock::new();
    if let Some(exists) = EXISTS.get() {
        return *exists;
    }
    let mut command = Command::new(ffmpeg_path());
    command.arg("-version").stderr(Stdio::null());
    let found = match output_within(&mut command, Duration::from_secs(5)) {
        Ok(output) => Some(output.status.success()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Some(false),
        Err(_) => None,
    };
    match found {
        Some(found) => {
            let _ = EXISTS.set(found);
            found
        }
        None => false,
    }
}

/// `draw_relevant_frame_from`: `ffmpeg -i <input> <video_preview_arguments> -`, capturing stdout.
pub fn video_preview(input: &Path) -> Result<Vec<u8>> {
    let mut command = Command::new(ffmpeg_path());
    command.arg("-i").arg(input).args(VIDEO_PREVIEW_ARGUMENTS).arg("-").stderr(Stdio::piped());
    let output = output_within(&mut command, FFMPEG_TIMEOUT).map_err(|error| match error.kind() {
        std::io::ErrorKind::TimedOut => Error::Preview(error.to_string()),
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
/// is an `ErrorKind::TimedOut` error naming the program. Stdin is closed and stdout captured;
/// stderr is captured only when the caller pipes it.
///
/// The child runs in its own process group. ffmpeg's preview graph uses `loop=-1`, and some
/// builds exit the parent while a descendant keeps stdout open, so joining the pipe reader
/// would wait forever. The deadline covers that reader too, and kills the whole group.
pub fn output_within(command: &mut Command, timeout: Duration) -> std::io::Result<Output> {
    let program = command.get_program().to_string_lossy().into_owned();
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.stdin(Stdio::null()).stdout(Stdio::piped()).spawn()?;
    // Drain the pipes while waiting, so a chatty child can't stall on a full pipe.
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let deadline = Instant::now() + timeout;
    let status = match wait_until(&mut child, deadline)? {
        Some(status) => status,
        None => {
            stop_child(&mut child);
            release(child, stdout, stderr);
            return Err(stuck(&program, timeout));
        }
    };
    match take_pipes(stdout, stderr, deadline) {
        Ok((stdout, stderr)) => Ok(Output { status, stdout, stderr }),
        Err((stdout, stderr)) => {
            stop_child(&mut child);
            release(child, stdout, stderr);
            Err(stuck(&program, timeout))
        }
    }
}

const REAP_GRACE: Duration = Duration::from_millis(500);

fn stuck(program: &str, timeout: Duration) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::TimedOut, format!("{program} timed out after {timeout:?}"))
}

fn stop_child(child: &mut Child) {
    #[cfg(unix)]
    {
        // SAFETY: the child was started with `process_group(0)`, so its pid is the process
        // group id. A negative pid signals that group and not this process.
        let _ = unsafe { kill(-(child.id() as i32), SIGKILL) };
    }
    let _ = child.kill();
}

#[cfg(unix)]
const SIGKILL: i32 = 9;

#[cfg(unix)]
unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

/// After the caller's kill: reap and join pipes within the grace period. A child still
/// running (uninterruptible I/O can outlive SIGKILL) and any reader a descendant is still
/// holding move to a detached thread, so this returns while `wait` and the joins finish.
fn release(mut child: Child, stdout: Pipe, stderr: Pipe) {
    let child = if child_reaped(&mut child) { None } else { Some(child) };
    let (stdout, stderr) = match take_pipes(stdout, stderr, Instant::now() + REAP_GRACE) {
        Ok(_) => (None, None),
        Err(pipes) => (Some(pipes.0), Some(pipes.1)),
    };
    detach_unfinished(child, stdout, stderr);
}

fn child_reaped(child: &mut Child) -> bool {
    match wait_until(child, Instant::now() + REAP_GRACE) {
        Ok(Some(_)) => true,
        Ok(None) => {
            let _ = child.kill();
            matches!(child.try_wait(), Ok(Some(_)))
        }
        Err(_) => false,
    }
}

fn detach_unfinished(child: Option<Child>, stdout: Option<Pipe>, stderr: Option<Pipe>) {
    if child.is_none() && stdout.is_none() && stderr.is_none() {
        return;
    }
    // Detach the reaper. It ends when the child exits and the pipes close.
    let _ = std::thread::Builder::new().name("media-process-reaper".into()).spawn(move || {
        if let Some(mut child) = child {
            let _ = child.wait();
        }
        if let Some(stdout) = stdout {
            let _ = stdout.join();
        }
        if let Some(stderr) = stderr {
            let _ = stderr.join();
        }
    });
}

type Pipe = std::thread::JoinHandle<Vec<u8>>;

fn take_pipes(stdout: Pipe, stderr: Pipe, deadline: Instant) -> std::result::Result<(Vec<u8>, Vec<u8>), (Pipe, Pipe)> {
    if !pipe_finished(&stdout, deadline) || !pipe_finished(&stderr, deadline) {
        return Err((stdout, stderr));
    }
    Ok((stdout.join().unwrap_or_default(), stderr.join().unwrap_or_default()))
}

fn pipe_finished(handle: &Pipe, deadline: Instant) -> bool {
    while !handle.is_finished() {
        let now = Instant::now();
        if now >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5).min(deadline - now));
    }
    true
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

    #[test]
    fn output_within_stops_a_writer_that_outlives_the_child() {
        let started = Instant::now();
        let mut command = Command::new("sh");
        // The shell exits immediately. The background loop inherits its stdout and would
        // otherwise keep the pipe reader joined forever.
        command.arg("-c").arg("(while true; do echo x; done) & exit 0");
        let error = output_within(&mut command, Duration::from_millis(300)).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        assert!(error.to_string().contains("sh"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(5), "took {:?}", started.elapsed());
    }

    #[cfg(unix)]
    #[test]
    fn detach_reaps_a_child_that_ignores_sigterm() {
        const SIGTERM: i32 = 15;
        let mut command = Command::new("sh");
        command.arg("-c").arg("trap '' TERM; echo ready; exec sleep 30");
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        let pid = child.id();
        struct Kill(u32);
        impl Drop for Kill {
            fn drop(&mut self) {
                // SAFETY: this pid is the child's process group, started with `process_group(0)`.
                unsafe { kill(-(self.0 as i32), SIGKILL) };
            }
        }
        let kill_group = Kill(pid);
        let mut stdout = child.stdout.take().unwrap();
        let mut ready = [0u8; 6];
        std::io::Read::read_exact(&mut stdout, &mut ready).unwrap();
        assert_eq!(&ready, b"ready\n");
        // SAFETY: `pid` is the child we just spawned.
        unsafe { kill(pid as i32, SIGTERM) };
        assert!(child.try_wait().unwrap().is_none(), "the child ignored SIGTERM");
        let stdout = drain(Some(stdout));
        let stderr = drain(child.stderr.take());
        let started = Instant::now();
        detach_unfinished(Some(child), Some(stdout), Some(stderr));
        assert!(started.elapsed() < Duration::from_millis(500), "detach blocked for {:?}", started.elapsed());
        drop(kill_group);
        let deadline = Instant::now() + Duration::from_secs(2);
        while Path::new(&format!("/proc/{pid}")).exists() {
            assert!(Instant::now() < deadline, "pid {pid} was not reaped");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
