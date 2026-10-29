//! Turbo Pascal's `Overlay` unit, faked.
//!
//! Overlays let DOS programs bigger than 640 KB run: units compiled with
//! `{$O+}` lived in a `.OVR` file and were loaded into a small buffer when
//! called. A Rust program has all its code in memory and no 640 KB limit,
//! so there is nothing to load. These routines only do what a program can
//! see: they report success in `OvrResult`, and remember the settings the
//! program makes so it reads back what it set.

use std::sync::atomic::{AtomicI16, AtomicI32, AtomicU8, Ordering};

/// `ovrOk` and the other values of `OvrResult`, as in TP.
pub const OVR_OK: i16 = 0;
pub const OVR_ERROR: i16 = -1;
pub const OVR_NOT_FOUND: i16 = -2;
pub const OVR_NO_MEMORY: i16 = -3;
pub const OVR_IO_ERROR: i16 = -4;
pub const OVR_NO_EMS_DRIVER: i16 = -5;
pub const OVR_NO_EMS_MEMORY: i16 = -6;

// Statics, like Crt's switches: settings any thread may read.
static OVR_RESULT: AtomicI16 = AtomicI16::new(OVR_OK);
static OVR_FILE_MODE: AtomicU8 = AtomicU8::new(0);
/// TP's default buffer was the size of the largest overlaid unit; 64 KB
/// is a plausible figure for `OvrGetBuf` to start from.
static OVR_BUF: AtomicI32 = AtomicI32::new(65_536);
static OVR_RETRY: AtomicI32 = AtomicI32::new(0);

/// `OvrInit(name)`: open the overlay file. There is none to open, and none
/// needed, so this always succeeds.
pub fn ovr_init(_name: impl Into<crate::string::PString>) {
    OVR_RESULT.store(OVR_OK, Ordering::Relaxed);
}

/// `OvrInitEMS`: copy the overlays into EMS memory. Succeeds too.
pub fn ovr_init_ems() {
    OVR_RESULT.store(OVR_OK, Ordering::Relaxed);
}

/// `OvrSetBuf(size)`: the size of the overlay buffer, remembered for
/// `OvrGetBuf`.
pub fn ovr_set_buf(size: impl Into<i32>) {
    OVR_BUF.store(size.into(), Ordering::Relaxed);
    OVR_RESULT.store(OVR_OK, Ordering::Relaxed);
}

/// `OvrGetBuf`.
pub fn ovr_get_buf() -> i32 {
    OVR_BUF.load(Ordering::Relaxed)
}

/// `OvrClearBuf`: empty the buffer. It's always empty.
pub fn ovr_clear_buf() {
    OVR_RESULT.store(OVR_OK, Ordering::Relaxed);
}

/// `OvrSetRetry(size)` (TP 6): the size of the "probation area",
/// remembered for `OvrGetRetry`.
pub fn ovr_set_retry(size: impl Into<i32>) {
    OVR_RETRY.store(size.into(), Ordering::Relaxed);
}

/// `OvrGetRetry` (TP 6).
pub fn ovr_get_retry() -> i32 {
    OVR_RETRY.load(Ordering::Relaxed)
}

/// The `OvrResult` variable.
pub fn ovr_result() -> i16 {
    OVR_RESULT.load(Ordering::Relaxed)
}

/// `OvrResult := n` (programs sometimes reset it themselves).
pub fn set_ovr_result(n: impl Into<i32>) {
    OVR_RESULT.store(n.into() as i16, Ordering::Relaxed);
}

/// The `OvrFileMode` variable: how `OvrInit` opens the file. Remembered.
pub fn ovr_file_mode() -> u8 {
    OVR_FILE_MODE.load(Ordering::Relaxed)
}

pub fn set_ovr_file_mode(n: impl Into<i32>) {
    OVR_FILE_MODE.store(n.into() as u8, Ordering::Relaxed);
}

/// `OvrLoadCount` and `OvrTrapCount`: how many times an overlay was loaded,
/// or a call into one was caught. Nothing is ever loaded, so 0.
pub fn ovr_load_count() -> u16 {
    0
}

pub fn ovr_trap_count() -> u16 {
    0
}

/// `OvrLoadCount := n` and `OvrTrapCount := n` (TP programs reset them to
/// measure a stretch of code). Accepted; they stay 0.
pub fn set_ovr_load_count(_n: impl Into<i32>) {}

pub fn set_ovr_trap_count(_n: impl Into<i32>) {}
