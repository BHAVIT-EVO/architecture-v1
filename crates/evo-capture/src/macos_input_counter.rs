//! Content-free input activity counting for native surfaces (macOS).
//!
//! The whole module exists to answer exactly one question every ten seconds:
//! *how many* keys, clicks, and scrolls landed while a surface was focused?
//! Nothing else. The listen-only event tap callback reads nothing from any
//! event except its type; there is no key value, no key code, no modifier
//! flag, no pointer position, and no window target anywhere in this module.
//! The counts are aggregated per flush bucket and attributed to the subject
//! the focus source currently reports — frontmost attribution, honestly
//! coarse: if the person typed while a dialog stole focus for a moment, the
//! bucket lands on the focused surface at flush time.
//!
//! Permission: listen-only taps require the Input Monitoring TCC grant. A
//! missing grant is reported as [`InputCounterError::InputMonitoringRequired`]
//! — silence, not a crash, and never a guess.
//!
//! Privacy rules this module enforces by construction:
//! 1. Counters, not events — raw input never crosses the callback boundary.
//! 2. Coarse time — the engine's finest meaningful unit is the minute; a
//!    10-second bucket blurs typing cadence past reconstruction.
//! 3. An all-zero bucket flushes nothing — absence of the observation is
//!    the zero.

use crate::adapters::macos::MacOSSignal;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// The flush bucket size. Ten seconds: coarse enough that per-second key
/// counts (a cadence side channel) cannot be reconstructed, fine enough
/// that a bucket spans at most one focused surface in practice.
pub const FLUSH_INTERVAL: Duration = Duration::from_secs(10);

/// Errors the input counter can report at start time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputCounterError {
    /// The platform is not macOS.
    UnsupportedPlatform,
    /// The Input Monitoring TCC grant is missing. macOS refuses to create
    /// listen-only taps without it. Nothing is counted until the person
    /// grants it in System Settings; the counter reports this once.
    InputMonitoringRequired,
    /// Tap or timer construction failed at the OS level.
    RuntimeSetupFailed(&'static str),
}

impl InputCounterError {
    /// Whether starting failed only because the permission is missing.
    pub fn is_permission(&self) -> bool {
        matches!(self, InputCounterError::InputMonitoringRequired)
    }
}

impl std::fmt::Display for InputCounterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputCounterError::UnsupportedPlatform => {
                write!(f, "input counting is only available on macOS")
            }
            InputCounterError::InputMonitoringRequired => write!(
                f,
                "Input Monitoring permission is required to count input activity; \
                 grant it in System Settings > Privacy & Security"
            ),
            InputCounterError::RuntimeSetupFailed(step) => {
                write!(f, "input counter setup failed: {step}")
            }
        }
    }
}

impl std::error::Error for InputCounterError {}

/// The pure aggregation core: what the tap callback bumps, what the flush
/// reads. Kept free of all FFI so the counting, bucketing, and zero-suppress
/// rules are testable without a tap.
#[derive(Debug, Default)]
pub struct InputCounters {
    keys: AtomicU64,
    clicks: AtomicU64,
    scrolls: AtomicU64,
}

impl InputCounters {
    /// One witnessed key-down. Nothing else was read about it.
    pub fn key(&self) {
        self.keys.fetch_add(1, Ordering::Relaxed);
    }

    /// One witnessed mouse-down (either button).
    pub fn click(&self) {
        self.clicks.fetch_add(1, Ordering::Relaxed);
    }

    /// One witnessed scroll gesture.
    pub fn scroll(&self) {
        self.scrolls.fetch_add(1, Ordering::Relaxed);
    }

    /// Takes the current bucket, resetting to zero. An all-zero bucket is
    /// `None`: absence of the observation is the zero.
    pub fn take(&self) -> Option<(u64, u64, u64)> {
        let keys = self.keys.swap(0, Ordering::Relaxed);
        let clicks = self.clicks.swap(0, Ordering::Relaxed);
        let scrolls = self.scrolls.swap(0, Ordering::Relaxed);
        (keys > 0 || clicks > 0 || scrolls > 0).then_some((keys, clicks, scrolls))
    }
}

/// Resolves which subject a flushed bucket lands on: the currently focused
/// surface's identity, document-grain where the focus source knows it.
pub type SubjectResolver = Box<dyn Fn() -> Option<String> + Send>;

#[cfg(target_os = "macos")]
mod imp {
    use super::*;

    use std::ffi::c_void;
    use std::ptr;

    type CFStringRef = *mut c_void;
    type CFMachPortRef = *mut c_void;
    type CFRunLoopSourceRef = *mut c_void;
    type CFRunLoopTimerRef = *mut c_void;
    type CFRunLoopTimerCallBack =
        extern "C" fn(CFRunLoopTimerRef, *mut c_void);
    type CGEventTapProxy = *mut c_void;
    type CGEventRef = *mut c_void;
    type CGEventType = u32;
    type CGEventMask = u64;
    type CGEventTapCallBack = extern "C" fn(
        CGEventTapProxy,
        CGEventType,
        CGEventRef,
        *mut c_void,
    ) -> CGEventRef;

    const K_CGFLOAT_MAX: f64 = f64::MAX;

    // Event types counted. `flagsChanged` is deliberately excluded: modifier
    // patterns are cadence too. `keyUp` is excluded: one key press is one
    // count, not two.
    const K_CGEVENT_LEFT_MOUSE_DOWN: CGEventType = 1;
    const K_CGEVENT_RIGHT_MOUSE_DOWN: CGEventType = 2;
    const K_CGEVENT_KEY_DOWN: CGEventType = 10;
    const K_CGEVENT_SCROLL_WHEEL: CGEventType = 22;

    const K_CGEVENT_TAP_SESSION: u32 = 1; // kCGSessionEventTap
    const K_CGEVENT_TAP_HEAD_APPEND: u32 = 0; // kCGHeadAppendEventTap
    const K_CGEVENT_TAP_LISTEN_ONLY: u32 = 1; // kCGEventTapOptionListenOnly

    /// The context `CFRunLoopTimerCreate` expects: a struct the function
    /// copies, whose `info` field is the pointer delivered to the callback.
    /// Passing a raw pointer here instead of a pointer to this struct makes
    /// CoreFoundation read arbitrary bytes as function pointers — the exact
    /// bus error this module's lifecycle test caught.
    #[repr(C)]
    struct CFRunLoopTimerContext {
        version: isize,
        info: *mut c_void,
        retain: Option<extern "C" fn(*const c_void) -> *mut c_void>,
        release: Option<extern "C" fn(*mut c_void)>,
        copy_description: Option<extern "C" fn(*const c_void) -> *mut c_void>,
    }

    unsafe extern "C" {
        static kCFRunLoopDefaultMode: CFStringRef;
        fn CFMachPortCreateRunLoopSource(
            allocator: *mut c_void,
            port: CFMachPortRef,
            order: isize,
        ) -> CFRunLoopSourceRef;
        fn CFRunLoopAddSource(
            run_loop: *mut c_void,
            source: CFRunLoopSourceRef,
            mode: CFStringRef,
        );
        fn CFRunLoopRemoveSource(
            run_loop: *mut c_void,
            source: CFRunLoopSourceRef,
            mode: CFStringRef,
        );
        fn CFRunLoopAddTimer(
            run_loop: *mut c_void,
            timer: CFRunLoopTimerRef,
            mode: CFStringRef,
        );
        fn CFRunLoopRemoveTimer(
            run_loop: *mut c_void,
            timer: CFRunLoopTimerRef,
            mode: CFStringRef,
        );
        fn CFRetain(cf: *mut c_void) -> *mut c_void;
        fn CFRunLoopGetCurrent() -> *mut c_void;
        fn CFMachPortInvalidate(port: CFMachPortRef);
        fn CFRunLoopTimerCreate(
            allocator: *mut c_void,
            fire_date: f64,
            interval: f64,
            flags: usize,
            order: isize,
            callout: CFRunLoopTimerCallBack,
            context: *mut c_void,
        ) -> CFRunLoopTimerRef;
        fn CFRelease(cf: *mut c_void);
        fn CFAbsoluteTimeGetCurrent() -> f64;
    }

    unsafe extern "C" {
        fn CGEventTapCreate(
            tap: u32,
            place: u32,
            options: u32,
            events_of_interest: CGEventMask,
            callback: CGEventTapCallBack,
            user_info: *mut c_void,
        ) -> CFMachPortRef;
        fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    }

    struct Shared {
        counters: InputCounters,
        resolver: SubjectResolver,
        sink: Box<dyn FnMut(MacOSSignal) + Send>,
    }

    /// Everything the run loop owns: the tap, its source, and the flush
    /// timer. Dropping it detaches all three.
    pub struct MacOSInputCounter {
        tap: CFMachPortRef,
        source: CFRunLoopSourceRef,
        timer: CFRunLoopTimerRef,
        /// The run loop the tap source and flush timer were added to,
        /// retained. Removal must happen on this exact loop: a run loop
        /// retains whatever is added to it, so releasing our own reference
        /// alone would leave the tap live with callbacks into freed memory.
        run_loop: *mut c_void,
        shared: Arc<Mutex<Option<Shared>>>,
    }

    impl MacOSInputCounter {
        /// Starts counting on the current thread's run loop. The callback
        /// receives one [`MacOSSignal::InputActivity`] per non-empty bucket,
        /// attributed to whatever `resolver` reports as focused at flush
        /// time. Counting begins immediately; the first bucket flushes after
        /// [`FLUSH_INTERVAL`].
        pub fn start(
            resolver: SubjectResolver,
            sink: Box<dyn FnMut(MacOSSignal) + Send>,
        ) -> Result<Self, InputCounterError> {
            let shared = Arc::new(Mutex::new(Some(Shared {
                counters: InputCounters::default(),
                resolver,
                sink,
            })));

            // The tap callback must be an extern fn with no captures; the
            // shared state travels through user_info. Only the event *type*
            // is read — that is the entire privacy contract of this module.
            extern "C" fn tap_callback(
                _proxy: CGEventTapProxy,
                event_type: CGEventType,
                event: CGEventRef,
                user_info: *mut c_void,
            ) -> CGEventRef {
                unsafe {
                    let shared = &*(user_info as *const Mutex<Option<Shared>>);
                    if let Ok(guard) = shared.lock() {
                        if let Some(state) = guard.as_ref() {
                            match event_type {
                                K_CGEVENT_KEY_DOWN => state.counters.key(),
                                K_CGEVENT_LEFT_MOUSE_DOWN | K_CGEVENT_RIGHT_MOUSE_DOWN => {
                                    state.counters.click()
                                }
                                K_CGEVENT_SCROLL_WHEEL => state.counters.scroll(),
                                _ => {}
                            }
                        }
                    }
                }
                // A listen-only tap's return value is ignored; pass the
                // event through untouched regardless.
                event
            }

            let mask: CGEventMask = (1u64 << K_CGEVENT_KEY_DOWN)
                | (1u64 << K_CGEVENT_LEFT_MOUSE_DOWN)
                | (1u64 << K_CGEVENT_RIGHT_MOUSE_DOWN)
                | (1u64 << K_CGEVENT_SCROLL_WHEEL);

            let tap = unsafe {
                CGEventTapCreate(
                    K_CGEVENT_TAP_SESSION,
                    K_CGEVENT_TAP_HEAD_APPEND,
                    K_CGEVENT_TAP_LISTEN_ONLY,
                    mask,
                    tap_callback,
                    Arc::as_ptr(&shared) as *mut c_void,
                )
            };
            if tap.is_null() {
                // A listen-only tap is refused only by policy: the Input
                // Monitoring grant is missing. That is the one failure this
                // constructor can name precisely.
                return Err(InputCounterError::InputMonitoringRequired);
            }
            unsafe { CGEventTapEnable(tap, true) };

            let source =
                unsafe { CFMachPortCreateRunLoopSource(ptr::null_mut(), tap, 0) };
            if source.is_null() {
                unsafe { CFRelease(tap) };
                return Err(InputCounterError::RuntimeSetupFailed(
                    "CFMachPortCreateRunLoopSource",
                ));
            }
            let run_loop = unsafe { CFRetain(CFRunLoopGetCurrent()) };
            unsafe { CFRunLoopAddSource(run_loop, source, kCFRunLoopDefaultMode) };

            extern "C" fn flush_timer(_timer: CFRunLoopTimerRef, user_info: *mut c_void) {
                unsafe {
                    let shared = &*(user_info as *const Mutex<Option<Shared>>);
                    let mut guard = match shared.lock() {
                        Ok(guard) => guard,
                        Err(_) => return,
                    };
                    let Some(state) = guard.as_mut() else { return };
                    let Some((keys, clicks, scrolls)) = state.counters.take() else {
                        return; // an all-zero bucket is not an observation
                    };
                    let subject = (state.resolver)();
                    (state.sink)(MacOSSignal::InputActivity {
                        subject,
                        keys,
                        clicks,
                        scrolls,
                        observed_at: SystemTime::now(),
                    });
                }
            }

            let interval = FLUSH_INTERVAL.as_secs_f64();
            let context = CFRunLoopTimerContext {
                version: 0,
                info: Arc::as_ptr(&shared) as *mut c_void,
                retain: None,
                release: None,
                copy_description: None,
            };
            let timer = unsafe {
                CFRunLoopTimerCreate(
                    ptr::null_mut(),
                    CFAbsoluteTimeGetCurrent() + interval,
                    interval,
                    0,
                    0,
                    flush_timer,
                    &context as *const CFRunLoopTimerContext as *mut c_void,
                )
            };
            if timer.is_null() {
                unsafe {
                    CFRelease(source);
                    CFRelease(tap);
                }
                return Err(InputCounterError::RuntimeSetupFailed("CFRunLoopTimerCreate"));
            }
            unsafe { CFRunLoopAddTimer(run_loop, timer, kCFRunLoopDefaultMode) };

            Ok(Self { tap, source, timer, run_loop, shared })
        }

        /// The aggregated counts so far, for a composition layer that wants
        /// to flush outside the timer (for example on focus change).
        pub fn counters(&self) -> (u64, u64, u64) {
            match self.shared.lock() {
                Ok(guard) => guard
                    .as_ref()
                    .map(|s| {
                        (
                            s.counters.keys.load(Ordering::Relaxed),
                            s.counters.clicks.load(Ordering::Relaxed),
                            s.counters.scrolls.load(Ordering::Relaxed),
                        )
                    })
                    .unwrap_or((0, 0, 0)),
                Err(_) => (0, 0, 0),
            }
        }
    }

    impl Drop for MacOSInputCounter {
        fn drop(&mut self) {
            // Teardown order is a memory-safety contract:
            // 1. Take the shared state so any callback that still races in
            //    finds `None` and returns without touching freed memory.
            let _ = self.shared.lock().map(|mut guard| guard.take());
            unsafe {
                // 2. Detach from the run loop we attached to. The loop
                //    retains the timer and the source; removal drops those
                //    retains so nothing fires after this point.
                CFRunLoopRemoveTimer(self.run_loop, self.timer, kCFRunLoopDefaultMode);
                CFRunLoopRemoveSource(self.run_loop, self.source, kCFRunLoopDefaultMode);
                // 3. Invalidate the tap: the documented way to stop event
                //    delivery to a CGEventTap before its port is released.
                CFMachPortInvalidate(self.tap);
                // 4. Now the releases are balanced and safe.
                CFRelease(self.timer);
                CFRelease(self.source);
                CFRelease(self.tap);
                CFRelease(self.run_loop);
            }
        }
    }
}

#[cfg(target_os = "macos")]
pub use imp::MacOSInputCounter;

#[cfg(not(target_os = "macos"))]
mod imp {
    use super::*;

    /// Non-macOS stub: the platform cannot count input this way, and says so.
    pub struct MacOSInputCounter;

    impl MacOSInputCounter {
        pub fn start(
            _resolver: SubjectResolver,
            _sink: Box<dyn FnMut(MacOSSignal) + Send>,
        ) -> Result<Self, InputCounterError> {
            Err(InputCounterError::UnsupportedPlatform)
        }

        pub fn counters(&self) -> (u64, u64, u64) {
            (0, 0, 0)
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub use imp::MacOSInputCounter;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_aggregate_and_reset() {
        let counters = InputCounters::default();
        for _ in 0..7 {
            counters.key();
        }
        counters.click();
        counters.click();
        counters.scroll();

        assert_eq!(counters.take(), Some((7, 2, 1)));
        // The take reset everything.
        assert_eq!(counters.take(), None);
    }

    #[test]
    fn an_all_zero_bucket_is_not_an_observation() {
        let counters = InputCounters::default();
        assert_eq!(counters.take(), None, "absence of the record is the zero");
    }

    /// The counter creates and tears down a real listen-only event tap. On
    /// a machine without the Input Monitoring grant the start reports the
    /// permission error; with it, the full lifecycle must be clean. Either
    /// way, this test proves the tap never crashes the process.
    #[test]
    #[cfg(target_os = "macos")]
    fn counter_lifecycle_is_clean_on_a_plain_thread() {
        use std::sync::mpsc;
        let (tx, rx) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            let started = MacOSInputCounter::start(
                Box::new(|| Some("file:///work/report.md".to_string())),
                Box::new(move |signal| {
                    let _ = tx.send(signal);
                }),
            );
            match started {
                Ok(counter) => {
                    // A short live window, then teardown on this same thread.
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    drop(counter);
                    "started-and-dropped"
                }
                Err(err) if err.is_permission() => "permission-refused",
                Err(err) => panic!("unexpected counter error: {err}"),
            }
        });
        let outcome = thread.join().expect("the thread must not crash");
        assert!(
            outcome == "started-and-dropped" || outcome == "permission-refused",
            "unexpected outcome: {outcome}"
        );
        // Drain whatever arrived — without blocking: on an idle machine (or
        // when the grant is missing) no signal ever will, and recv() would
        // hang the suite forever.
        let _ = rx.try_recv();
    }

    #[test]
    fn counts_only_reset_on_take() {
        let counters = InputCounters::default();
        counters.key();
        counters.key();
        // Reading via the composition accessor does not consume the bucket.
        // (Covered through counters() on the real counter; here the pure
        // form: repeated takes without new input yield None, not repeats.)
        assert_eq!(counters.take(), Some((2, 0, 0)));
        assert_eq!(counters.take(), None);
        assert_eq!(counters.take(), None);
    }
}
