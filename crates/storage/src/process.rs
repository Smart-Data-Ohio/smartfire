//! Producing variant and preview bytes: `ActiveStorage::Transformers::Vips` (image_processing
//! 1.14) and `ActiveStorage::Previewer::VideoPreviewer`.

use std::io::Read;
use std::path::Path;
#[cfg(not(unix))]
use std::process::ExitStatus;
use std::process::{Child, Command, Output, Stdio};
use std::sync::{OnceLock, mpsc};
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

    let output = tempfile::Builder::new()
        .prefix("image_processing")
        .suffix(&format!(".{format}"))
        .tempfile()?;
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
                return Err(Error::InvalidVariation(
                    "combine_options is not supported".into(),
                ));
            }
            (_, argument) if blank(argument) => {}
            ("resize_to_limit", Value::Array(args)) if args.len() == 2 => {
                let dimension = |v: Option<&Value>| match v {
                    None | Some(Value::Nil) => Ok(None),
                    Some(Value::Int(n)) => Ok(Some(*n as i32)),
                    Some(other) => Err(Error::InvalidVariation(format!(
                        "resize_to_limit argument {other:?}"
                    ))),
                };
                let (width, height) = (dimension(args.first())?, dimension(args.get(1))?);
                if width.is_none() && height.is_none() {
                    return Err(Error::InvalidVariation(
                        "either width or height must be specified".into(),
                    ));
                }
                operations.push((width, height));
            }
            (name, argument) => {
                return Err(Error::InvalidVariation(format!(
                    "unsupported transformation {name}: {argument:?}"
                )));
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
    command
        .arg("-i")
        .arg(input)
        .args(VIDEO_PREVIEW_ARGUMENTS)
        .arg("-")
        .stderr(Stdio::piped());
    let output =
        output_within(&mut command, FFMPEG_TIMEOUT).map_err(|error| match error.kind() {
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
///
/// On Unix the leader is observed with `waitid(WNOWAIT)`, which does not reap. `wait()` runs
/// only after that observation, so it cannot block, and `killpg` happens while the pid is
/// still unreaped and therefore not reusable. `ECHILD` means the leader was already collected.
pub fn output_within(command: &mut Command, timeout: Duration) -> std::io::Result<Output> {
    let program = command.get_program().to_string_lossy().into_owned();
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()?;
    collect_within(&program, child, timeout)
}

/// Watch a spawned child until it exits or `timeout` elapses. `ECHILD` from `waitid` means
/// the leader was already collected; that path does not signal it.
fn collect_within(program: &str, mut child: Child, timeout: Duration) -> std::io::Result<Output> {
    // Drain the pipes while waiting, so a chatty child can't stall on a full pipe.
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let deadline = Instant::now() + timeout;
    let mut pause = Duration::from_millis(1);
    let saw_exit = loop {
        match poll_exit(&mut child) {
            Ok(true) => break true,
            Ok(false) => {}
            Err(err) if child_collected(&err) => {
                release(child, stdout, stderr);
                return Err(err);
            }
            Err(err) => {
                stop_child(&mut child);
                release(child, stdout, stderr);
                return Err(err);
            }
        }
        let now = Instant::now();
        if now >= deadline {
            break false;
        }
        std::thread::sleep(pause.min(deadline - now));
        pause = (pause * 2).min(Duration::from_millis(50));
    };
    if !saw_exit {
        stop_child(&mut child);
        release(child, stdout, stderr);
        return Err(stuck(program, timeout));
    }
    match take_pipes(stdout, stderr, deadline) {
        Ok((stdout, stderr)) => match child.wait() {
            Ok(status) => Ok(Output {
                status,
                stdout,
                stderr,
            }),
            Err(err) if child_collected(&err) => Err(err),
            Err(err) => {
                stop_child(&mut child);
                handoff(Some(child), None, None);
                Err(err)
            }
        },
        Err((stdout, stderr)) => {
            stop_child(&mut child);
            release(child, stdout, stderr);
            Err(stuck(program, timeout))
        }
    }
}

const REAP_GRACE: Duration = Duration::from_millis(500);

fn stuck(program: &str, timeout: Duration) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        format!("{program} timed out after {timeout:?}"),
    )
}

fn stop_child(child: &mut Child) {
    #[cfg(unix)]
    {
        let pid = child.id();
        #[cfg(test)]
        SIGNALED_PIDS.lock().expect("signaled pids").push(pid);
        // SAFETY: this child was spawned with `process_group(0)` and has not been waited,
        // so `pid` is still that process group and cannot have been reused. `killpg` takes
        // the positive group id.
        let _ = unsafe { libc::killpg(pid as libc::pid_t, libc::SIGKILL) };
    }
    let _ = child.kill();
}

#[cfg(all(unix, test))]
fn pid_reaped(pid: u32) -> bool {
    // SAFETY: signal 0 checks whether `pid` exists and does not deliver a signal.
    let rc = unsafe { libc::kill(pid as libc::pid_t, 0) };
    rc != 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}

/// After the caller's kill: reap and join pipes within the grace period. A child still
/// running (uninterruptible I/O can outlive SIGKILL) and any reader a descendant is still
/// holding go to the process-wide reaper, so this returns while that cleanup finishes.
fn release(mut child: Child, stdout: Pipe, stderr: Pipe) {
    let child = if child_reaped(&mut child) {
        None
    } else {
        Some(child)
    };
    let (stdout, stderr) = match take_pipes(stdout, stderr, Instant::now() + REAP_GRACE) {
        Ok(_) => (None, None),
        Err(pipes) => (Some(pipes.0), Some(pipes.1)),
    };
    handoff(child, stdout, stderr);
}

fn child_reaped(child: &mut Child) -> bool {
    #[cfg(unix)]
    {
        reap_when_observed(child, Instant::now() + REAP_GRACE)
    }
    #[cfg(not(unix))]
    {
        match wait_until(child, Instant::now() + REAP_GRACE) {
            Ok(Some(_)) => true,
            Ok(None) => {
                let _ = child.kill();
                matches!(child.try_wait(), Ok(Some(_)))
            }
            Err(_) => false,
        }
    }
}

/// `true` once `waitid` has reported the exit and `wait()` has collected it. A leader that is
/// still not waitable stays unreaped so the caller can hand it to the reaper.
#[cfg(unix)]
fn reap_when_observed(child: &mut Child, deadline: Instant) -> bool {
    let mut pause = Duration::from_millis(1);
    loop {
        match leader_waitable(child.id()) {
            Ok(true) => {
                let _ = child.wait();
                return true;
            }
            Err(err) if err.raw_os_error() == Some(libc::ECHILD) => return true,
            Ok(false) | Err(_) => {}
        }
        let now = Instant::now();
        if now >= deadline {
            return false;
        }
        std::thread::sleep(pause.min(deadline - now));
        pause = (pause * 2).min(Duration::from_millis(50));
    }
}

struct ReapJob {
    child: Option<Child>,
    stdout: Option<Pipe>,
    stderr: Option<Pipe>,
}

fn handoff(child: Option<Child>, stdout: Option<Pipe>, stderr: Option<Pipe>) {
    if child.is_none() && stdout.is_none() && stderr.is_none() {
        return;
    }
    // Readers stuck on pipes held by descendants that escaped the group are an accepted residual.
    let job = ReapJob {
        child,
        stdout,
        stderr,
    };
    match reaper_sender() {
        Some(sender) => {
            if let Err(mpsc::SendError(job)) = sender.send(job) {
                fallback_wait(job);
            }
        }
        None => fallback_wait(job),
    }
}

fn fallback_wait(mut job: ReapJob) {
    let Some(mut child) = job.child.take() else {
        return;
    };
    #[cfg(unix)]
    if matches!(leader_waitable(child.id()), Ok(true)) {
        let _ = child.wait();
    }
    #[cfg(not(unix))]
    {
        let _ = child.wait();
    }
}

fn reaper_sender() -> Option<&'static mpsc::Sender<ReapJob>> {
    static REAPER: OnceLock<mpsc::Sender<ReapJob>> = OnceLock::new();
    static START: std::sync::Mutex<()> = std::sync::Mutex::new(());
    if let Some(sender) = REAPER.get() {
        return Some(sender);
    }
    // A failed spawn is not stored, so a later call can try again. The mutex keeps a single reaper.
    let _guard = START.lock().unwrap_or_else(|err| err.into_inner());
    if let Some(sender) = REAPER.get() {
        return Some(sender);
    }
    let (sender, receiver) = mpsc::channel();
    std::thread::Builder::new()
        .name("media-process-reaper".into())
        .spawn(move || reap_loop(receiver))
        .ok()?;
    Some(REAPER.get_or_init(|| sender))
}

fn reap_loop(receiver: mpsc::Receiver<ReapJob>) {
    let mut jobs = Vec::new();
    loop {
        let incoming = if jobs.is_empty() {
            match receiver.recv() {
                Ok(job) => Some(job),
                Err(_) => return,
            }
        } else {
            match receiver.recv_timeout(Duration::from_millis(50)) {
                Ok(job) => Some(job),
                Err(mpsc::RecvTimeoutError::Timeout) => None,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    std::thread::sleep(Duration::from_millis(50));
                    None
                }
            }
        };
        let mut drained = 0;
        if let Some(job) = incoming {
            jobs.push(job);
            drained = 1;
        }
        // Bound the drain so a stream of new jobs cannot starve the exit poll below.
        while drained < REAP_DRAIN_LIMIT {
            match receiver.try_recv() {
                Ok(job) => {
                    jobs.push(job);
                    drained += 1;
                }
                Err(_) => break,
            }
        }
        jobs.retain_mut(|job| !job.poll());
    }
}

const REAP_DRAIN_LIMIT: usize = 64;

impl ReapJob {
    /// Reap a child only after its exit has been observed, and join readers that have finished.
    /// `true` when nothing is left.
    fn poll(&mut self) -> bool {
        self.reap_if_exited();
        join_finished(&mut self.stdout);
        join_finished(&mut self.stderr);
        self.child.is_none() && self.stdout.is_none() && self.stderr.is_none()
    }

    fn reap_if_exited(&mut self) {
        #[cfg(unix)]
        {
            let Some(pid) = self.child.as_ref().map(Child::id) else {
                return;
            };
            match leader_waitable(pid) {
                Ok(true) => {
                    if self
                        .child
                        .take()
                        .is_some_and(|mut child| child.wait().is_ok())
                    {
                        note_reaped(pid);
                    }
                }
                Err(err) if err.raw_os_error() == Some(libc::ECHILD) => self.child = None,
                _ => {}
            }
        }
        #[cfg(not(unix))]
        {
            let reaped = self
                .child
                .as_mut()
                .is_some_and(|child| matches!(child.try_wait(), Ok(Some(_))));
            if reaped {
                self.child = None;
            }
        }
    }
}

fn join_finished(pipe: &mut Option<Pipe>) {
    if let Some(handle) = pipe.take_if(|handle| handle.is_finished()) {
        let _ = handle.join();
    }
}

type Pipe = std::thread::JoinHandle<Vec<u8>>;

fn take_pipes(
    stdout: Pipe,
    stderr: Pipe,
    deadline: Instant,
) -> std::result::Result<(Vec<u8>, Vec<u8>), (Pipe, Pipe)> {
    if !pipe_finished(&stdout, deadline) || !pipe_finished(&stderr, deadline) {
        return Err((stdout, stderr));
    }
    Ok((
        stdout.join().unwrap_or_default(),
        stderr.join().unwrap_or_default(),
    ))
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

fn poll_exit(child: &mut Child) -> std::io::Result<bool> {
    #[cfg(unix)]
    {
        leader_waitable(child.id())
    }
    #[cfg(not(unix))]
    {
        Ok(child.try_wait()?.is_some())
    }
}

#[cfg(unix)]
fn child_collected(err: &std::io::Error) -> bool {
    err.raw_os_error() == Some(libc::ECHILD)
}

#[cfg(not(unix))]
fn child_collected(_err: &std::io::Error) -> bool {
    false
}

/// `true` when `waitid` reports an exited child and leaves it unreaped (`WNOWAIT`).
#[cfg(unix)]
fn leader_waitable(pid: u32) -> std::io::Result<bool> {
    loop {
        // SAFETY: `siginfo_t` is a C struct whose zero value is the "no child" state `waitid` reads.
        let mut info = unsafe { std::mem::zeroed::<libc::siginfo_t>() };
        // SAFETY: `info` is a zeroed `siginfo_t`. `P_PID` names `pid`. `WNOWAIT` leaves the
        // child unreaped so a later `wait` can collect it. A zero `si_pid` means `WNOHANG`
        // found nothing waitable.
        let rc = unsafe {
            libc::waitid(
                libc::P_PID,
                pid as libc::id_t,
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if rc == 0 {
            // SAFETY: `waitid` returned 0. Either it wrote a sigchld `siginfo` or, with
            // `WNOHANG` and no status, left the zeroed `si_pid` untouched.
            return Ok(unsafe { info.si_pid() } != 0);
        }
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EINTR) {
            continue;
        }
        return Err(err);
    }
}

#[cfg(test)]
static REAPED_BY_REAPER: std::sync::Mutex<Vec<u32>> = std::sync::Mutex::new(Vec::new());

#[cfg(all(unix, test))]
static SIGNALED_PIDS: std::sync::Mutex<Vec<u32>> = std::sync::Mutex::new(Vec::new());

fn note_reaped(pid: u32) {
    #[cfg(test)]
    REAPED_BY_REAPER.lock().expect("reaped pid list").push(pid);
    #[cfg(not(test))]
    let _ = pid;
}

#[cfg(test)]
fn reaped_by_reaper(pid: u32) -> bool {
    REAPED_BY_REAPER
        .lock()
        .expect("reaped pid list")
        .contains(&pid)
}

/// The child's exit status, or `None` while it's still running at `deadline`.
#[cfg(not(unix))]
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
        command
            .args(["-c", "echo out; echo err >&2"])
            .stderr(Stdio::piped());
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
        command.arg("-c").arg(format!(
            "echo $$ > {}; exec sleep 30",
            pid_file.path().display()
        ));
        let error = output_within(&mut command, Duration::from_millis(300)).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "took {:?}",
            started.elapsed()
        );
        let pid = std::fs::read_to_string(pid_file.path()).unwrap();
        assert!(
            !Path::new(&format!("/proc/{}", pid.trim())).exists(),
            "the child is still running"
        );
    }

    #[test]
    fn output_within_stops_a_writer_that_outlives_the_child() {
        let started = Instant::now();
        let mut command = Command::new("sh");
        // The shell exits immediately. The background loop inherits its stdout and would
        // otherwise keep the pipe reader joined forever.
        command
            .arg("-c")
            .arg("(while true; do echo x; done) & exit 0");
        let error = output_within(&mut command, Duration::from_millis(300)).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        assert!(error.to_string().contains("sh"), "{error}");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "took {:?}",
            started.elapsed()
        );
    }

    #[cfg(unix)]
    #[test]
    fn reaper_reaps_a_child_that_ignores_sigterm() {
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg("trap '' TERM; echo ready; exec sleep 30");
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let pid = child.id();
        // Drop signals only if we still own the leader. Disarm before the reaper waits.
        struct GroupKill {
            pid: u32,
            armed: bool,
        }
        impl GroupKill {
            fn fire(&mut self) {
                if self.armed {
                    // SAFETY: `process_group(0)` made this pid the group id, and the leader
                    // has not been reaped. `killpg` takes that positive group id.
                    unsafe { libc::killpg(self.pid as libc::pid_t, libc::SIGKILL) };
                    self.armed = false;
                }
            }
        }
        impl Drop for GroupKill {
            fn drop(&mut self) {
                if self.armed {
                    // SAFETY: still armed, so the leader has not been handed to the reaper.
                    unsafe { libc::killpg(self.pid as libc::pid_t, libc::SIGKILL) };
                }
            }
        }
        let mut guard = GroupKill { pid, armed: true };
        let mut stdout = child.stdout.take().unwrap();
        let mut ready = [0u8; 6];
        std::io::Read::read_exact(&mut stdout, &mut ready).unwrap();
        assert_eq!(&ready, b"ready\n");
        // SAFETY: `pid` is the unreaped child we just spawned. Signal 0 is not used; SIGTERM is ignored.
        unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) };
        assert!(!leader_waitable(pid).unwrap(), "the child ignored SIGTERM");
        guard.fire();
        assert!(
            !pid_reaped(pid),
            "the leader stays unreaped until the reaper waits"
        );
        assert!(
            reaper_sender().is_some(),
            "the background reaper did not start"
        );
        assert!(
            !reaped_by_reaper(pid),
            "the synchronous path must not record the reap"
        );
        let stdout = drain(Some(stdout));
        let stderr = drain(child.stderr.take());
        handoff(Some(child), Some(stdout), Some(stderr));
        let deadline = Instant::now() + Duration::from_secs(30);
        while !reaped_by_reaper(pid) {
            assert!(
                Instant::now() < deadline,
                "background reaper did not reap pid {pid}"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[cfg(unix)]
    #[test]
    fn output_within_does_not_signal_after_echild() {
        let mut command = Command::new("sh");
        command.args(["-c", "echo out"]);
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let pid = child.id();
        let ready = Instant::now() + Duration::from_secs(5);
        loop {
            match leader_waitable(pid) {
                Ok(true) => break,
                Ok(false) => {}
                Err(err) => panic!("waitid failed before the reap: {err}"),
            }
            assert!(Instant::now() < ready, "child {pid} did not exit");
            std::thread::sleep(Duration::from_millis(1));
        }
        // Collect this pid only, so the `Child` is stale and `waitid` returns ECHILD.
        let mut status = 0;
        let rc = loop {
            // SAFETY: `pid` is the unreaped leader this test spawned in its own group.
            // `waitpid` collects that pid and no other child.
            let rc = unsafe { libc::waitpid(pid as libc::pid_t, &mut status, 0) };
            if rc < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                continue;
            }
            break rc;
        };
        assert_eq!(rc, pid as libc::pid_t, "waitpid did not collect {pid}");
        let started = Instant::now();
        let error = collect_within("sh", child, Duration::from_secs(5)).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ECHILD), "{error}");
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "took {:?}",
            started.elapsed()
        );
        assert!(
            !SIGNALED_PIDS.lock().expect("signaled pids").contains(&pid),
            "signaled pid {pid} after ECHILD"
        );
    }
}
