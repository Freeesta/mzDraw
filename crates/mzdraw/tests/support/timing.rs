//! The clock of the scaling tests: the CPU time of the running thread, not the wall clock.
//!
//! A scaling test compares the cost of a small and of a large input. On a shared machine the wall clock also counts the time the
//! thread spent waiting for a core, which has nothing to do with the code and lands on one of the two runs at random. The CPU time
//! of the thread does not count it. Where the thread clock is not available (not Linux 64-bit) the wall clock is used, so the tests
//! still run, with `best_of` taking out what noise it can. Included by `#[path]`, so it is in no library and in no binary.

#![allow(dead_code)]

use std::time::{Duration, Instant};

#[cfg(all(target_os = "linux", target_pointer_width = "64"))]
mod clock {
    #[repr(C)]
    struct Timespec {
        sec: i64,
        nsec: i64,
    }
    unsafe extern "C" {
        fn clock_gettime(clock: i32, out: *mut Timespec) -> i32;
    }
    /// `CLOCK_THREAD_CPUTIME_ID` of Linux.
    const THREAD_CPUTIME: i32 = 3;
    pub fn thread_cpu_ns() -> Option<u128> {
        let mut t = Timespec { sec: 0, nsec: 0 };
        // SAFETY: `t` is a valid `timespec` of this platform, written by the call and read after it succeeded.
        let ok = unsafe { clock_gettime(THREAD_CPUTIME, &mut t) } == 0;
        ok.then(|| t.sec as u128 * 1_000_000_000 + t.nsec as u128)
    }
}

#[cfg(not(all(target_os = "linux", target_pointer_width = "64")))]
mod clock {
    pub fn thread_cpu_ns() -> Option<u128> {
        None
    }
}

/// A stopwatch on the CPU time of the calling thread (the wall clock where that is not available). Start and read it on one thread.
pub struct Watch(Option<u128>, Instant);

impl Watch {
    pub fn start() -> Watch {
        Watch(clock::thread_cpu_ns(), Instant::now())
    }
    pub fn elapsed(&self) -> Duration {
        match (self.0, clock::thread_cpu_ns()) {
            (Some(a), Some(b)) => Duration::from_nanos(b.saturating_sub(a) as u64),
            _ => self.1.elapsed(),
        }
    }
}

/// The least CPU time of `runs` runs of `f` after `warm` runs that are not timed (caches, the allocator and the page tables are
/// the same for the small and for the large input): noise only ever adds time, so the least is the cost of the code.
pub fn best_of(warm: usize, runs: usize, mut f: impl FnMut()) -> Duration {
    for _ in 0..warm {
        f();
    }
    (0..runs)
        .map(|_| {
            let w = Watch::start();
            f();
            w.elapsed()
        })
        .min()
        .expect("at least one run")
}
