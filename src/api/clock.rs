use std::cmp::Ordering;
use std::convert::TryInto;
use std::fmt;
use std::mem;
use std::sys::c;
use std::time::Duration;

use winapi::{
    shared::minwindef::{FILETIME, LPFILETIME, DWORD},
    um::sysinfoapi::GetSystemTimePreciseAsFileTime,
};

use core::hash::{Hash, Hasher};

const NANOS_PER_SEC: u64 = 1_000_000_000;
const INTERVALS_PER_SEC: u64 = NANOS_PER_SEC / 100;

#[derive(Copy, Clone)]
pub struct KrazyKraigTime {
    t: FILETIME,
}

const INTERVALS_TO_UNIX_EPOCH: u64 = 11_644_473_600 * INTERVALS_PER_SEC;

pub const UNIX_EPOCH: KrazyKraigTime = KrazyKraigTime {
    t: FILETIME {
        dwLowDateTime: INTERVALS_TO_UNIX_EPOCH as u32,
        dwHighDateTime: (INTERVALS_TO_UNIX_EPOCH >> 32) as u32,
    },
};

impl KrazyKraigTime {
    pub fn now() -> KrazyKraigTime {
        unsafe {
            let mut t: FILETIME = FILETIME::default();
            GetSystemTimePreciseAsFileTime(&mut t);
            Self(t)
        }
    }

    fn from_intervals(intervals: i64) -> KrazyKraigTime {
        KrazyKraigTime {
            t: FILETIME {
                dwLowDateTime: intervals as DWORD,
                dwHighDateTime: (intervals >> 32) as DWORD,
            },
        }
    }

    fn intervals(&self) -> i64 {
        (self.t.dwLowDateTime as i64) | ((self.t.dwHighDateTime as i64) << 32)
    }

    pub fn sub_time(&self, other: &KrazyKraigTime) -> Result<Duration, Duration> {
        let me = self.intervals();
        let other = other.intervals();
        if me >= other {
            Ok(intervals2dur((me - other) as u64))
        } else {
            Err(intervals2dur((other - me) as u64))
        }
    }

    pub fn checked_add_duration(&self, other: &Duration) -> Option<KrazyKraigTime> {
        let intervals = self.intervals().checked_add(checked_dur2intervals(other)?)?;
        Some(KrazyKraigTime::from_intervals(intervals))
    }

    pub fn checked_sub_duration(&self, other: &Duration) -> Option<KrazyKraigTime> {
        let intervals = self.intervals().checked_sub(checked_dur2intervals(other)?)?;
        Some(KrazyKraigTime::from_intervals(intervals))
    }
}

impl PartialEq for KrazyKraigTime {
    fn eq(&self, other: &KrazyKraigTime) -> bool {
        self.intervals() == other.intervals()
    }
}

impl Eq for KrazyKraigTime {}

impl PartialOrd for KrazyKraigTime {
    fn partial_cmp(&self, other: &KrazyKraigTime) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for KrazyKraigTime {
    fn cmp(&self, other: &KrazyKraigTime) -> Ordering {
        self.intervals().cmp(&other.intervals())
    }
}

impl fmt::Debug for KrazyKraigTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KrazyKraigTime").field("intervals", &self.intervals()).finish()
    }
}

impl From<FILETIME> for KrazyKraigTime {
    fn from(t: FILETIME) -> KrazyKraigTime {
        KrazyKraigTime { t }
    }
}

impl Hash for KrazyKraigTime {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.intervals().hash(state)
    }
}

fn checked_dur2intervals(dur: &Duration) -> Option<i64> {
    dur.as_secs()
        .checked_mul(INTERVALS_PER_SEC)?
        .checked_add(dur.subsec_nanos() as u64 / 100)?
        .try_into()
        .ok()
}

fn intervals2dur(intervals: u64) -> Duration {
    Duration::new(intervals / INTERVALS_PER_SEC, ((intervals % INTERVALS_PER_SEC) * 100) as u32)
}
