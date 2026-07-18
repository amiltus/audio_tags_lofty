use std::ffi::{CStr, CString, c_char};
use std::fs;
use std::path::Path;
use std::ptr;

use lofty::config::ParseOptions;
use lofty::file::TaggedFileExt;
use lofty::picture::PictureType;
use lofty::probe::Probe;

/// Owned result returned across the FFI boundary.
///
/// A successful no-artwork result has a null `data` pointer, zero length, and
/// a null `error` pointer. Every other result owns its data/error until freed
/// with `lofty_free_artwork_result`.
#[repr(C)]
pub struct ArtworkResult {
    pub data: *mut u8,
    pub len: usize,
    pub error: *mut c_char,
}

impl ArtworkResult {
    fn artwork(data: Vec<u8>) -> *mut Self {
        let len = data.len();
        let data = Box::into_raw(data.into_boxed_slice()) as *mut u8;
        Box::into_raw(Box::new(Self {
            data,
            len,
            error: ptr::null_mut(),
        }))
    }

    fn no_artwork() -> *mut Self {
        Box::into_raw(Box::new(Self {
            data: ptr::null_mut(),
            len: 0,
            error: ptr::null_mut(),
        }))
    }

    fn error(message: impl Into<String>) -> *mut Self {
        let error = CString::new(message.into())
            .unwrap_or_else(|_| CString::new("Audio artwork extraction failed").unwrap())
            .into_raw();
        Box::into_raw(Box::new(Self {
            data: ptr::null_mut(),
            len: 0,
            error,
        }))
    }
}

/// Extracts front-cover bytes from a bounded local audio file.
///
/// This Rust-level entry point exists for deterministic tests and benchmarks;
/// production callers use the C ABI below.
pub fn extract_front_artwork(
    path: &Path,
    max_input_bytes: u64,
    max_artwork_bytes: u64,
) -> Result<Option<Vec<u8>>, String> {
    if max_input_bytes == 0 || max_artwork_bytes == 0 {
        return Err("Input and artwork limits must be positive".to_string());
    }

    let metadata = fs::metadata(path)
        .map_err(|error| format!("Unable to read audio file metadata: {error}"))?;
    if !metadata.is_file() {
        return Err("Audio input must be a regular file".to_string());
    }
    if metadata.len() > max_input_bytes {
        return Err(format!(
            "Audio input requires {} bytes, exceeding the configured {}-byte limit",
            metadata.len(),
            max_input_bytes,
        ));
    }

    let tagged = Probe::open(path)
        .map_err(|error| format!("Unable to open audio file: {error}"))?
        .guess_file_type()
        .map_err(|error| format!("Unable to identify audio file: {error}"))?
        .options(ParseOptions::new().read_cover_art(true))
        .read()
        .map_err(|error| format!("Unable to read audio tags: {error}"))?;

    let pictures: Vec<_> = tagged
        .tags()
        .iter()
        .flat_map(|tag| tag.pictures())
        .collect();
    let picture = select_picture(&pictures);
    let Some(picture) = picture else {
        return Ok(None);
    };

    let data = picture.data();
    if data.is_empty() {
        return Ok(None);
    }
    if data.len() as u64 > max_artwork_bytes {
        return Err(format!(
            "Embedded artwork requires {} bytes, exceeding the configured {}-byte limit",
            data.len(),
            max_artwork_bytes,
        ));
    }
    Ok(Some(data.to_vec()))
}

fn select_picture<'a>(
    pictures: &[&'a lofty::picture::Picture],
) -> Option<&'a lofty::picture::Picture> {
    pictures
        .iter()
        .copied()
        .find(|picture| picture.pic_type() == PictureType::CoverFront)
        .or_else(|| pictures.first().copied())
}

/// Extracts the front cover (or first cover if no front cover is present) from
/// a local audio path. The function performs no network I/O and does not
/// decode audio samples.
///
/// # Safety
/// - `path` must be a non-null, null-terminated UTF-8 C string.
/// - The caller must free the returned result exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lofty_extract_front_artwork(
    path: *const c_char,
    max_input_bytes: u64,
    max_artwork_bytes: u64,
) -> *mut ArtworkResult {
    if path.is_null() {
        return ArtworkResult::error("Audio path must not be null");
    }
    let path = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(path) if !path.is_empty() => Path::new(path),
        Ok(_) => return ArtworkResult::error("Audio path must not be empty"),
        Err(_) => return ArtworkResult::error("Audio path must be valid UTF-8"),
    };
    match extract_front_artwork(path, max_input_bytes, max_artwork_bytes) {
        Ok(Some(data)) => ArtworkResult::artwork(data),
        Ok(None) => ArtworkResult::no_artwork(),
        Err(error) => ArtworkResult::error(error),
    }
}

/// Frees a result returned from `lofty_extract_front_artwork`. Passing null is
/// a no-op.
///
/// # Safety
/// - `result` must be a value returned by this library.
/// - It must be freed at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lofty_free_artwork_result(result: *mut ArtworkResult) {
    if result.is_null() {
        return;
    }
    let result = unsafe { Box::from_raw(result) };
    if !result.data.is_null() && result.len > 0 {
        let _ = unsafe { Vec::from_raw_parts(result.data, result.len, result.len) };
    }
    if !result.error.is_null() {
        let _ = unsafe { CString::from_raw(result.error) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lofty::picture::{MimeType, Picture};

    fn fixture(name: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }

    #[test]
    fn rejects_empty_limits_before_opening_input() {
        let error = extract_front_artwork(Path::new("missing.mp3"), 0, 1)
            .expect_err("zero input limit must be rejected");
        assert!(error.contains("limits must be positive"));
    }

    #[test]
    fn no_artwork_result_is_a_successful_empty_result() {
        let result = ArtworkResult::no_artwork();
        unsafe {
            assert!((*result).data.is_null());
            assert_eq!((*result).len, 0);
            assert!((*result).error.is_null());
            lofty_free_artwork_result(result);
        }
    }

    #[test]
    fn prefers_front_cover_over_the_first_picture() {
        let back =
            Picture::new_unchecked(PictureType::CoverBack, Some(MimeType::Jpeg), None, vec![1]);
        let front =
            Picture::new_unchecked(PictureType::CoverFront, Some(MimeType::Jpeg), None, vec![2]);
        let selected = select_picture(&[&back, &front]).expect("a picture should be selected");

        assert_eq!(selected.data(), &[2]);
    }

    #[test]
    fn extracts_artwork_from_a_valid_local_file() {
        let artwork =
            extract_front_artwork(&fixture("with_front_artwork.mp3"), 1024 * 1024, 1024 * 1024)
                .expect("fixture should parse")
                .expect("fixture should have artwork");

        assert!(artwork.starts_with(&[0xFF, 0xD8, 0xFF]));
    }

    #[test]
    fn treats_no_artwork_as_a_successful_empty_value() {
        let artwork =
            extract_front_artwork(&fixture("without_artwork.mp3"), 1024 * 1024, 1024 * 1024)
                .expect("fixture should parse");

        assert!(artwork.is_none());
    }

    #[test]
    fn rejects_malformed_audio_without_returning_artwork() {
        let error = extract_front_artwork(&fixture("malformed.audio"), 1024 * 1024, 1024 * 1024)
            .expect_err("malformed input must fail closed");

        assert!(!error.is_empty());
    }

    #[test]
    fn rejects_input_and_artwork_over_the_configured_limits() {
        let path = fixture("with_front_artwork.mp3");
        let input_bytes = fs::metadata(&path).unwrap().len();
        let input_error = extract_front_artwork(&path, input_bytes - 1, 1024 * 1024)
            .expect_err("input should exceed the limit");
        assert!(input_error.contains("Audio input requires"));

        let artwork_error = extract_front_artwork(&path, 1024 * 1024, 1)
            .expect_err("artwork should exceed the limit");
        assert!(artwork_error.contains("Embedded artwork requires"));
    }
}
