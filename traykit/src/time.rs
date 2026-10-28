//! The local wall-clock time, without a date crate.

/// Local calendar time, to the millisecond. Local rather than UTC because
/// what it labels (a log line, a file name) is read by a human next to a
/// clock on the same wall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub millis: u16,
}

impl LocalTime {
    pub fn now() -> Self {
        imp::now()
    }
}

#[cfg(windows)]
mod imp {
    use super::LocalTime;
    use windows::Win32::System::SystemInformation::GetLocalTime;

    pub fn now() -> LocalTime {
        let t = unsafe { GetLocalTime() };
        LocalTime {
            year: t.wYear,
            month: t.wMonth as u8,
            day: t.wDay as u8,
            hour: t.wHour as u8,
            minute: t.wMinute as u8,
            second: t.wSecond as u8,
            millis: t.wMilliseconds,
        }
    }
}

#[cfg(unix)]
mod imp {
    use super::LocalTime;

    pub fn now() -> LocalTime {
        let dur = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = dur.as_secs() as libc::time_t;
        // localtime_r honours the current zone and DST.
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        unsafe {
            libc::localtime_r(&secs, &mut tm);
        }
        LocalTime {
            year: (tm.tm_year + 1900) as u16,
            month: (tm.tm_mon + 1) as u8,
            day: tm.tm_mday as u8,
            hour: tm.tm_hour as u8,
            minute: tm.tm_min as u8,
            second: tm.tm_sec as u8,
            millis: dur.subsec_millis() as u16,
        }
    }
}
