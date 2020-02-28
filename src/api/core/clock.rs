use std::cmp::Ordering;
use std::convert::TryInto;
use std::fmt;
use std::time::Duration;
use std::ops::{Add, AddAssign, Sub, SubAssign};

use winapi::{
    shared::minwindef::{FILETIME, DWORD},
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

#[derive(Debug)]
pub struct SystemTimeError(Duration);

pub const UNIX_EPOCH: KrazyKraigTime = KrazyKraigTime {
    t: FILETIME {
        dwLowDateTime: INTERVALS_TO_UNIX_EPOCH as u32,
        dwHighDateTime: (INTERVALS_TO_UNIX_EPOCH >> 32) as u32,
    },
};

impl KrazyKraigTime {
    pub fn now() -> KrazyKraigTime {
        unsafe {
            let mut t = FILETIME::default();
            GetSystemTimePreciseAsFileTime(&mut t);
            Self { t }
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

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimeStamp(KrazyKraigTime);

impl fmt::Debug for TimeStamp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl Add<Duration> for TimeStamp {
    type Output = TimeStamp;

    /// # Panics
    ///
    /// This function may panic if the resulting point in time cannot be represented by the
    /// underlying data structure. See [`checked_add`] for a version without panic.
    ///
    /// [`checked_add`]: ../../std/time/struct.TimeStamp.html#method.checked_add
    fn add(self, dur: Duration) -> TimeStamp {
        TimeStamp(self.0.checked_add_duration(&dur).expect("overflow when adding duration to instant"))
    }
}

impl AddAssign<Duration> for TimeStamp {
    fn add_assign(&mut self, other: Duration) {
        self.0 = self.0.checked_add_duration(&other).expect("overflow when adding duration to instant");
        //self.0 = self.0 + other;
    }
}

impl Sub<Duration> for TimeStamp {
    type Output = TimeStamp;

    fn sub(self, dur: Duration) -> TimeStamp {
        TimeStamp(self.0.checked_sub_duration(&dur).expect("overflow when subtracting duration from instant"))
    }
}

impl SubAssign<Duration> for TimeStamp {
    fn sub_assign(&mut self, other: Duration) {
        self.0 = self.0.checked_sub_duration(&other).expect("overflow when subtracting duration from instant");
        //self.0 = self.0 - other;
    }
}

impl TimeStamp {
    pub const UNIX_EPOCH: TimeStamp = TimeStamp(UNIX_EPOCH);

    pub fn now() -> TimeStamp {
        Self(KrazyKraigTime::now())
    }

    pub fn duration_since(&self, earlier: TimeStamp) -> Result<Duration, SystemTimeError> {
        self.0.sub_time(&earlier.0).map_err(SystemTimeError)
    }

    pub fn elapsed(&self) -> Result<Duration, SystemTimeError> {
        TimeStamp::now().duration_since(*self)
    }

    pub fn checked_add(&self, duration: Duration) -> Option<TimeStamp> {
        self.0.checked_add_duration(&duration).map(TimeStamp)
    }

    pub fn checked_sub(&self, duration: Duration) -> Option<TimeStamp> {
        self.0.checked_sub_duration(&duration).map(TimeStamp)
    }
}

impl SystemTimeError {
    pub fn duration(&self) -> Duration {
        self.0
    }
}

impl std::error::Error for SystemTimeError {
    fn description(&self) -> &str {
        "other time was not earlier than self"
    }
}

impl fmt::Display for SystemTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "second time provided was later than self")
    }
}

/*
impl FromInner<SystemTime> for SystemTime {
    fn from_inner(time: SystemTime) -> SystemTime {
        SystemTime(time)
    }
}
*/