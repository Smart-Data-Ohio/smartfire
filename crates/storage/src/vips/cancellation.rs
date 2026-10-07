//! Cooperative cancellation for both libvips evaluation and its source's header reads.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::{Error, Image, Path, Result, VipsImage, c_char, c_int, c_void, g_object_unref};

#[cfg(feature = "test-support")]
type Stall = (StallPhase, Arc<AtomicBool>);

#[derive(Clone)]
pub struct Cancellation {
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
    #[cfg(feature = "test-support")]
    stall: Arc<Mutex<Option<Stall>>>,
}

impl Cancellation {
    pub fn new(timeout: Duration) -> Self {
        Self {
            deadline: Instant::now() + timeout,
            cancelled: Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "test-support")]
            stall: Arc::new(Mutex::new(None)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed) || Instant::now() >= self.deadline
    }

    pub fn check(&self) -> Result<()> {
        if self.is_cancelled() {
            Err(Error::Vips("image processing cancelled".into()))
        } else {
            Ok(())
        }
    }

    #[cfg(feature = "test-support")]
    pub(crate) fn stall_at(&self, phase: StallPhase, reached: Arc<AtomicBool>) {
        *self.stall.lock().unwrap() = Some((phase, reached));
    }

    #[cfg(feature = "test-support")]
    fn stall(&self, phase: StallPhase) {
        let held = self.stall.lock().ok().and_then(|stall| stall.clone());
        if let Some((at, reached)) = held
            && at == phase
        {
            reached.store(true, Ordering::Relaxed);
            // This never finishes without the same cancellation check used by real work.
            while !self.is_cancelled() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}

#[cfg(feature = "test-support")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StallPhase {
    Header,
    Evaluation,
}

#[link(name = "vips")]
#[link(name = "gobject-2.0")]
unsafe extern "C" {
    fn vips_source_custom_new() -> *mut c_void;
    fn vips_image_set_progress(image: *mut VipsImage, progress: c_int);
    fn vips_image_set_kill(image: *mut VipsImage, kill: c_int);
    fn g_signal_connect_data(
        instance: *mut c_void,
        signal: *const c_char,
        handler: unsafe extern "C" fn(),
        data: *mut c_void,
        destroy: unsafe extern "C" fn(*mut c_void, *mut c_void),
        flags: c_int,
    ) -> std::ffi::c_ulong;
}

impl Image {
    pub fn cancel_on(&self, cancel: &Cancellation) -> Result<()> {
        cancel.check()?;
        unsafe {
            vips_image_set_progress(self.0, 1);
            // libvips sends progress on the most downstream image with progress enabled.
            // Each image owns its token until GObject releases the signal closure, including
            // when the operation cache retains the pipeline after Rust's Image is dropped.
            for signal in [c"preeval", c"eval"] {
                g_signal_connect_data(
                    self.0.cast(),
                    signal.as_ptr(),
                    std::mem::transmute::<
                        unsafe extern "C" fn(*mut VipsImage, *mut c_void, *mut c_void),
                        unsafe extern "C" fn(),
                    >(eval),
                    Box::into_raw(Box::new(cancel.clone())).cast(),
                    destroy::<Cancellation>,
                    0,
                );
            }
        }
        Ok(())
    }
}

unsafe extern "C" fn eval(image: *mut VipsImage, _: *mut c_void, data: *mut c_void) {
    let cancel = unsafe { &*data.cast::<Cancellation>() };
    #[cfg(feature = "test-support")]
    cancel.stall(StallPhase::Evaluation);
    if cancel.is_cancelled() {
        unsafe { vips_image_set_kill(image, 1) };
    }
}

pub(super) struct Source(pub(super) *mut c_void);

struct Input {
    file: Mutex<File>,
    cancel: Cancellation,
}

impl Source {
    pub(super) fn open(path: &Path, cancel: &Cancellation) -> Result<Self> {
        cancel.check()?;
        let file = File::open(path)?;
        if !file.metadata()?.is_file() {
            return Err(Error::Vips("image source is not a regular file".into()));
        }
        let source = Self(unsafe { vips_source_custom_new() });
        if source.0.is_null() {
            return Err(Error::Vips(super::take_error()));
        }
        let input = Arc::new(Input {
            file: Mutex::new(file),
            cancel: cancel.clone(),
        });
        unsafe {
            g_signal_connect_data(
                source.0,
                c"read".as_ptr(),
                std::mem::transmute::<
                    unsafe extern "C" fn(*mut c_void, *mut c_void, i64, *mut c_void) -> i64,
                    unsafe extern "C" fn(),
                >(read),
                Box::into_raw(Box::new(input.clone())).cast(),
                destroy::<Arc<Input>>,
                0,
            );
            g_signal_connect_data(
                source.0,
                c"seek".as_ptr(),
                std::mem::transmute::<
                    unsafe extern "C" fn(*mut c_void, i64, c_int, *mut c_void) -> i64,
                    unsafe extern "C" fn(),
                >(seek),
                Box::into_raw(Box::new(input)).cast(),
                destroy::<Arc<Input>>,
                0,
            );
        }
        Ok(source)
    }
}

impl Drop for Source {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { g_object_unref(self.0) };
        }
    }
}

unsafe extern "C" fn read(
    _: *mut c_void,
    buffer: *mut c_void,
    length: i64,
    data: *mut c_void,
) -> i64 {
    let input = unsafe { &*data.cast::<Arc<Input>>() };
    #[cfg(feature = "test-support")]
    input.cancel.stall(StallPhase::Header);
    if input.cancel.is_cancelled() || length < 0 || buffer.is_null() {
        return -1;
    }
    // Bound each read so large source requests return to the cancellation check regularly.
    let bytes = unsafe {
        std::slice::from_raw_parts_mut(buffer.cast::<u8>(), (length as usize).min(64 * 1024))
    };
    input
        .file
        .lock()
        .ok()
        .and_then(|mut file| file.read(bytes).ok())
        .map_or(-1, |n| n as i64)
}

unsafe extern "C" fn seek(_: *mut c_void, offset: i64, whence: c_int, data: *mut c_void) -> i64 {
    let input = unsafe { &*data.cast::<Arc<Input>>() };
    if input.cancel.is_cancelled() {
        return -1;
    }
    let position = match whence {
        0 if offset >= 0 => SeekFrom::Start(offset as u64),
        1 => SeekFrom::Current(offset),
        2 => SeekFrom::End(offset),
        _ => return -1,
    };
    input
        .file
        .lock()
        .ok()
        .and_then(|mut file| file.seek(position).ok())
        .and_then(|n| n.try_into().ok())
        .unwrap_or(-1)
}

unsafe extern "C" fn destroy<T>(data: *mut c_void, _: *mut c_void) {
    drop(unsafe { Box::from_raw(data.cast::<T>()) });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_callback_kills_pixel_evaluation() {
        // Enough scanlines for repeated progress callbacks: a tiny image can finish before
        // the sink checks the kill flag set by its final progress notification.
        super::super::init();
        let pixels = vec![0.0; 2048 * 2048];
        // A memory image has no cancellable source; only the eval signal can stop this write.
        let image = Image::wrap(unsafe {
            super::super::vips_image_new_matrix_from_array(
                2048,
                2048,
                pixels.as_ptr(),
                pixels.len() as c_int,
            )
        })
        .unwrap();
        let cancel = Cancellation::new(Duration::from_secs(10));
        image.cancel_on(&cancel).unwrap();
        cancel.cancel();
        let output = tempfile::Builder::new().suffix(".png").tempfile().unwrap();
        assert!(image.write_to_file(output.path()).is_err());
    }
}
