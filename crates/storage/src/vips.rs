//! Minimal libvips FFI covering exactly the calls ruby-vips makes for Active Storage:
//! the Vips image analyzer, and `ImageProcessing::Vips` `resize_to_limit` + format conversion.
//!
//! libvips must be the same version as the reference image's for variants to be byte-identical.

use std::ffi::{CStr, CString, c_char, c_double, c_int, c_void};
use std::path::Path;
use std::sync::Once;

use crate::{Error, Result};

#[repr(C)]
struct VipsImage {
    _private: [u8; 0],
}

const VIPS_ARGUMENT_REQUIRED: c_int = 1;
const VIPS_ARGUMENT_INPUT: c_int = 16;
const VIPS_ACCESS_SEQUENTIAL: c_int = 1;
const VIPS_SIZE_DOWN: c_int = 2;
const VIPS_PRECISION_INTEGER: c_int = 0;
const G_TYPE_INVALID: usize = 0;

#[link(name = "vips")]
#[link(name = "gobject-2.0")]
#[link(name = "glib-2.0")]
unsafe extern "C" {
    fn vips_init(argv0: *const c_char) -> c_int;
    fn vips_version_string() -> *const c_char;
    fn vips_block_untrusted_set(state: c_int);
    fn vips_operation_block_set(name: *const c_char, state: c_int);
    fn vips_error_buffer_copy() -> *mut c_char;
    fn vips_error_clear();
    fn vips_foreign_find_load(filename: *const c_char) -> *const c_char;
    fn vips_operation_new(name: *const c_char) -> *mut c_void;
    fn vips_object_get_argument_flags(object: *mut c_void, name: *const c_char) -> c_int;
    fn vips_image_new_from_file(name: *const c_char, ...) -> *mut VipsImage;
    fn vips_pngload(filename: *const c_char, out: *mut *mut VipsImage, ...) -> c_int;
    fn vips_jpegload(filename: *const c_char, out: *mut *mut VipsImage, ...) -> c_int;
    fn vips_gifload(filename: *const c_char, out: *mut *mut VipsImage, ...) -> c_int;
    fn vips_webpload(filename: *const c_char, out: *mut *mut VipsImage, ...) -> c_int;
    fn vips_image_new_matrix_from_array(width: c_int, height: c_int, array: *const c_double, size: c_int) -> *mut VipsImage;
    fn vips_image_set_double(image: *mut VipsImage, name: *const c_char, d: c_double);
    fn vips_image_get_width(image: *const VipsImage) -> c_int;
    fn vips_image_get_height(image: *const VipsImage) -> c_int;
    fn vips_image_get_typeof(image: *const VipsImage, name: *const c_char) -> usize;
    fn vips_image_get_as_string(image: *const VipsImage, name: *const c_char, out: *mut *mut c_char) -> c_int;
    fn vips_autorot(input: *mut VipsImage, out: *mut *mut VipsImage, ...) -> c_int;
    fn vips_thumbnail_image(input: *mut VipsImage, out: *mut *mut VipsImage, width: c_int, ...) -> c_int;
    fn vips_conv(input: *mut VipsImage, out: *mut *mut VipsImage, mask: *mut VipsImage, ...) -> c_int;
    fn vips_image_write_to_file(image: *mut VipsImage, name: *const c_char, ...) -> c_int;
    fn g_object_unref(object: *mut c_void);
    fn g_free(mem: *mut c_void);
}

static INIT: Once = Once::new();

/// `vips_init`, then the loader restrictions of `config/initializers/vips.rb`:
/// `Vips.block_untrusted(true)` and `Vips.block("VipsForeignLoadOpenslide", true)`.
pub fn init() {
    INIT.call_once(|| unsafe {
        if vips_init(c"campfire".as_ptr()) != 0 {
            panic!("vips_init failed: {}", take_error());
        }
        vips_block_untrusted_set(1);
        vips_operation_block_set(c"VipsForeignLoadOpenslide".as_ptr(), 1);
    });
}

pub fn version() -> String {
    init();
    unsafe { CStr::from_ptr(vips_version_string()).to_string_lossy().into_owned() }
}

pub struct Image(*mut VipsImage);

// libvips images are immutable once built and reference counted with atomic refcounts.
unsafe impl Send for Image {}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe { g_object_unref(self.0.cast()) }
    }
}

impl Image {
    /// Header-only until evaluated, with sequential access, strict error handling and at
    /// most one page. Use the loader matching the magic bytes, never a fallback loader.
    pub fn open_branding(path: &Path, content_type: &str) -> Result<Image> {
        init();
        let path = cstring(path)?;
        let mut out = std::ptr::null_mut();
        const FAIL_ON_ERROR: c_int = 2;
        macro_rules! load {
            ($loader:ident $(, $option:expr, $value:expr)*) => {
                unsafe {
                    $loader(
                        path.as_ptr(), &mut out,
                        c"access".as_ptr(), VIPS_ACCESS_SEQUENTIAL,
                        c"fail_on".as_ptr(), FAIL_ON_ERROR,
                        c"revalidate".as_ptr(), 1 as c_int,
                        $($option.as_ptr(), $value as c_int,)*
                        std::ptr::null::<c_char>(),
                    )
                }
            };
        }
        let status = match content_type {
            "image/png" => load!(vips_pngload),
            "image/jpeg" => load!(vips_jpegload),
            "image/gif" => load!(vips_gifload, c"page", 0, c"n", 1),
            "image/webp" => load!(vips_webpload, c"page", 0, c"n", 1),
            _ => return Err(Error::Vips("unsupported branding format".into())),
        };
        Image::wrap_out(status, out)
    }

    fn wrap(ptr: *mut VipsImage) -> Result<Image> {
        if ptr.is_null() { Err(Error::Vips(take_error())) } else { Ok(Image(ptr)) }
    }

    fn wrap_out(status: c_int, out: *mut VipsImage) -> Result<Image> {
        if status != 0 { Err(Error::Vips(take_error())) } else { Image::wrap(out) }
    }

    /// `Vips::Image.new_from_file(path, access: :sequential)`, as the image analyzer opens files.
    pub fn open_sequential(path: &Path) -> Result<Image> {
        init();
        let path = cstring(path)?;
        Image::wrap(unsafe {
            vips_image_new_from_file(path.as_ptr(), c"access".as_ptr(), VIPS_ACCESS_SEQUENTIAL, std::ptr::null::<c_char>())
        })
    }

    /// `ImageProcessing::Vips::Processor.load_image(path, page: 0)`: `page: 0` only reaches loaders
    /// that accept it (`Utils.select_valid_loader_options`), then `autorot`.
    pub fn load_for_processing(path: &Path) -> Result<Image> {
        init();
        let path = cstring(path)?;
        let image = if loader_accepts_page(&path) {
            unsafe { vips_image_new_from_file(path.as_ptr(), c"page".as_ptr(), 0 as c_int, std::ptr::null::<c_char>()) }
        } else {
            unsafe { vips_image_new_from_file(path.as_ptr(), std::ptr::null::<c_char>()) }
        };
        Image::wrap(image)?.autorot()
    }

    pub fn width(&self) -> i32 {
        unsafe { vips_image_get_width(self.0) }
    }

    pub fn height(&self) -> i32 {
        unsafe { vips_image_get_height(self.0) }
    }

    /// `image.get(name)` for string fields such as `exif-ifd0-Orientation`; `None` when absent.
    pub fn get_string(&self, name: &str) -> Option<String> {
        let name = CString::new(name).ok()?;
        unsafe {
            if vips_image_get_typeof(self.0, name.as_ptr()) == G_TYPE_INVALID {
                return None;
            }
            let mut out: *mut c_char = std::ptr::null_mut();
            if vips_image_get_as_string(self.0, name.as_ptr(), &mut out) != 0 {
                vips_error_clear();
                return None;
            }
            let value = CStr::from_ptr(out).to_string_lossy().into_owned();
            g_free(out.cast());
            Some(value)
        }
    }

    pub fn autorot(&self) -> Result<Image> {
        let mut out = std::ptr::null_mut();
        let status = unsafe { vips_autorot(self.0, &mut out, std::ptr::null::<c_char>()) };
        Image::wrap_out(status, out)
    }

    /// `resize_to_limit(width, height)`: `thumbnail_image(width, height:, size: :down,
    /// no_rotate: true)` followed by `conv(SHARPEN_MASK, precision: :integer)`.
    pub fn resize_to_limit(&self, width: Option<i32>, height: Option<i32>) -> Result<Image> {
        const MAX_COORD: i32 = 10_000_000;
        let (width, height) = (width.unwrap_or(MAX_COORD), height.unwrap_or(MAX_COORD));
        let mut thumbnail = std::ptr::null_mut();
        let status = unsafe {
            vips_thumbnail_image(
                self.0,
                &mut thumbnail,
                width as c_int,
                c"height".as_ptr(),
                height as c_int,
                c"size".as_ptr(),
                VIPS_SIZE_DOWN,
                c"no_rotate".as_ptr(),
                1 as c_int,
                std::ptr::null::<c_char>(),
            )
        };
        let thumbnail = Image::wrap_out(status, thumbnail)?;
        let mask = sharpen_mask()?;
        let mut sharpened = std::ptr::null_mut();
        let status = unsafe {
            vips_conv(thumbnail.0, &mut sharpened, mask.0, c"precision".as_ptr(), VIPS_PRECISION_INTEGER, std::ptr::null::<c_char>())
        };
        Image::wrap_out(status, sharpened)
    }

    /// `write_to_file(path)`: the saver and its defaults are picked from the extension.
    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        let path = cstring(path)?;
        let status = unsafe { vips_image_write_to_file(self.0, path.as_ptr(), std::ptr::null::<c_char>()) };
        if status != 0 { Err(Error::Vips(take_error())) } else { Ok(()) }
    }
}

/// `ImageProcessing::Vips::Processor::SHARPEN_MASK`: `new_from_array([[-1,-1,-1],[-1,32,-1],[-1,-1,-1]], 24)`.
fn sharpen_mask() -> Result<Image> {
    let values: [c_double; 9] = [-1.0, -1.0, -1.0, -1.0, 32.0, -1.0, -1.0, -1.0, -1.0];
    let mask = Image::wrap(unsafe { vips_image_new_matrix_from_array(3, 3, values.as_ptr(), 9) })?;
    unsafe {
        vips_image_set_double(mask.0, c"scale".as_ptr(), 24.0);
        vips_image_set_double(mask.0, c"offset".as_ptr(), 0.0);
    }
    Ok(mask)
}

/// Whether the loader libvips picks for `path` has an optional `page` input (ruby-vips'
/// `Introspect#optional_input`).
fn loader_accepts_page(path: &CStr) -> bool {
    unsafe {
        let loader = vips_foreign_find_load(path.as_ptr());
        if loader.is_null() {
            vips_error_clear();
            return false;
        }
        let operation = vips_operation_new(loader);
        if operation.is_null() {
            vips_error_clear();
            return false;
        }
        let flags = vips_object_get_argument_flags(operation, c"page".as_ptr());
        g_object_unref(operation);
        flags & VIPS_ARGUMENT_INPUT != 0 && flags & VIPS_ARGUMENT_REQUIRED == 0
    }
}

fn cstring(path: &Path) -> Result<CString> {
    use std::os::unix::ffi::OsStrExt;
    CString::new(path.as_os_str().as_bytes()).map_err(|_| Error::Vips("path contains a NUL byte".into()))
}

/// libvips' error buffer is process-wide; `vips_error_buffer_copy` takes and clears it under
/// libvips' lock, so concurrent calls don't read each other's errors.
fn take_error() -> String {
    unsafe {
        let copy = vips_error_buffer_copy();
        if copy.is_null() {
            return String::new();
        }
        let message = CStr::from_ptr(copy).to_string_lossy().trim_end().to_string();
        g_free(copy.cast());
        message
    }
}
