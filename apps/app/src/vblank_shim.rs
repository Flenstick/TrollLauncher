//! Emulates `DRM_IOCTL_WAIT_VBLANK` for drivers that reject it.
//!
//! WebKitGTK paces `requestAnimationFrame` and compositing with DRM vblank
//! waits. The proprietary NVIDIA driver answers them with `EOPNOTSUPP`, so
//! WebKit falls back to a fixed 60 Hz timer and the whole UI runs at 60 FPS
//! even on high refresh rate monitors. This module interposes the process-wide
//! `ioctl` symbol and, only when the kernel refuses a vblank wait, sleeps until
//! the next tick of the CRTC's real refresh rate instead.

use std::ffi::{c_int, c_ulong, c_void};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

const DRM_IOCTL_WAIT_VBLANK: c_ulong = 0xc018_643a;
const DRM_IOCTL_MODE_GETRESOURCES: c_ulong = 0xc040_64a0;
const DRM_IOCTL_MODE_GETCRTC: c_ulong = 0xc068_64a1;

const DRM_VBLANK_RELATIVE: u32 = 0x1;
const DRM_VBLANK_EVENT: u32 = 0x400_0000;
const DRM_VBLANK_SECONDARY: u32 = 0x2000_0000;
const DRM_VBLANK_HIGH_CRTC_MASK: u32 = 0x3e;
const DRM_VBLANK_HIGH_CRTC_SHIFT: u32 = 1;

#[repr(C)]
struct WaitVblank {
    kind: u32,
    sequence: u32,
    tval_sec: libc::c_long,
    tval_usec: libc::c_long,
}

#[repr(C)]
#[derive(Default)]
struct CardRes {
    fb_id_ptr: u64,
    crtc_id_ptr: u64,
    connector_id_ptr: u64,
    encoder_id_ptr: u64,
    count_fbs: u32,
    count_crtcs: u32,
    count_connectors: u32,
    count_encoders: u32,
    min_width: u32,
    max_width: u32,
    min_height: u32,
    max_height: u32,
}

#[repr(C)]
#[derive(Default)]
struct ModeInfo {
    clock: u32,
    hdisplay: u16,
    hsync_start: u16,
    hsync_end: u16,
    htotal: u16,
    hskew: u16,
    vdisplay: u16,
    vsync_start: u16,
    vsync_end: u16,
    vtotal: u16,
    vscan: u16,
    vrefresh: u32,
    flags: u32,
    kind: u32,
    name: [u8; 32],
}

#[repr(C)]
#[derive(Default)]
struct ModeCrtc {
    set_connectors_ptr: u64,
    count_connectors: u32,
    crtc_id: u32,
    fb_id: u32,
    x: u32,
    y: u32,
    gamma_size: u32,
    mode_valid: u32,
    mode: ModeInfo,
}

type IoctlFn = unsafe extern "C" fn(c_int, c_ulong, *mut c_void) -> c_int;

fn real_ioctl() -> IoctlFn {
    static REAL: OnceLock<usize> = OnceLock::new();
    let address = *REAL.get_or_init(|| unsafe {
        libc::dlsym(libc::RTLD_NEXT, c"ioctl".as_ptr()) as usize
    });
    unsafe { std::mem::transmute::<usize, IoctlFn>(address) }
}

/// Frame period in nanoseconds, 0 while unknown.
static PERIOD_NS: AtomicU64 = AtomicU64::new(0);

unsafe fn detect_period_ns(fd: c_int, crtc_index: usize) -> Option<u64> {
    let real = real_ioctl();
    let mut res = CardRes::default();
    if unsafe { real(fd, DRM_IOCTL_MODE_GETRESOURCES, (&raw mut res).cast()) }
        != 0
    {
        return None;
    }
    let count = res.count_crtcs as usize;
    if crtc_index >= count || count > 64 {
        return None;
    }
    let mut ids = vec![0u32; count];
    let mut res = CardRes {
        crtc_id_ptr: ids.as_mut_ptr() as u64,
        count_crtcs: count as u32,
        ..Default::default()
    };
    if unsafe { real(fd, DRM_IOCTL_MODE_GETRESOURCES, (&raw mut res).cast()) }
        != 0
    {
        return None;
    }
    let mut crtc = ModeCrtc {
        crtc_id: ids[crtc_index],
        ..Default::default()
    };
    if unsafe { real(fd, DRM_IOCTL_MODE_GETCRTC, (&raw mut crtc).cast()) } != 0
    {
        return None;
    }
    let mode = &crtc.mode;
    if crtc.mode_valid == 0
        || mode.htotal == 0
        || mode.vtotal == 0
        || mode.clock == 0
    {
        return None;
    }
    let refresh = f64::from(mode.clock) * 1000.0
        / (f64::from(mode.htotal) * f64::from(mode.vtotal));
    tracing::info!(
        "Emulating DRM vblank at {refresh:.3} Hz for CRTC {crtc_index}"
    );
    Some((1e9 / refresh) as u64)
}

fn monotonic_ns() -> u64 {
    let mut now = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &raw mut now) };
    now.tv_sec as u64 * 1_000_000_000 + now.tv_nsec as u64
}

unsafe fn emulate_wait_vblank(fd: c_int, wait: &mut WaitVblank) -> bool {
    if wait.kind & DRM_VBLANK_EVENT != 0 {
        return false;
    }
    let crtc_index = if wait.kind & DRM_VBLANK_SECONDARY != 0 {
        1
    } else {
        ((wait.kind & DRM_VBLANK_HIGH_CRTC_MASK) >> DRM_VBLANK_HIGH_CRTC_SHIFT)
            as usize
    };

    let mut period = PERIOD_NS.load(Ordering::Relaxed);
    if period == 0 {
        match unsafe { detect_period_ns(fd, crtc_index) } {
            Some(detected) => {
                PERIOD_NS.store(detected, Ordering::Relaxed);
                period = detected;
            }
            None => return false,
        }
    }

    let now = monotonic_ns();
    let tick = now / period;
    let target = if wait.kind & DRM_VBLANK_RELATIVE != 0 {
        tick + u64::from(wait.sequence)
    } else {
        u64::from(wait.sequence).max(tick)
    };

    let mut woke = now;
    if target > tick {
        let deadline = target * period;
        let ts = libc::timespec {
            tv_sec: (deadline / 1_000_000_000) as libc::time_t,
            tv_nsec: (deadline % 1_000_000_000) as libc::c_long,
        };
        while unsafe {
            libc::clock_nanosleep(
                libc::CLOCK_MONOTONIC,
                libc::TIMER_ABSTIME,
                &raw const ts,
                std::ptr::null_mut(),
            )
        } == libc::EINTR
        {}
        woke = deadline;
    }

    wait.sequence = target as u32;
    wait.tval_sec = (woke / 1_000_000_000) as libc::c_long;
    wait.tval_usec = ((woke % 1_000_000_000) / 1000) as libc::c_long;
    true
}

/// Process-wide `ioctl` override, exported from the executable so shared
/// libraries (libdrm, WebKitGTK) resolve to it. Everything except a refused
/// vblank wait is forwarded untouched.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ioctl(
    fd: c_int,
    request: c_ulong,
    arg: *mut c_void,
) -> c_int {
    let result = unsafe { real_ioctl()(fd, request, arg) };
    if result == 0
        || request != DRM_IOCTL_WAIT_VBLANK
        || arg.is_null()
        || std::io::Error::last_os_error().raw_os_error()
            != Some(libc::EOPNOTSUPP)
    {
        return result;
    }

    if unsafe { emulate_wait_vblank(fd, &mut *arg.cast::<WaitVblank>()) } {
        0
    } else {
        unsafe { *libc::__errno_location() = libc::EOPNOTSUPP };
        result
    }
}
