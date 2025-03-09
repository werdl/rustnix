use alloc::{
    string::String,
    format
};

pub mod pit;
pub mod rtc;

// return the time since boot in seconds
pub fn get_time_since_boot() -> f64 {
    pit::get_ticks() as f64 * pit::PIT_INTERVAL
}

// Get the current time from the RTC
pub fn get_time() -> String {
    let (second, minute, hour, day, month, year) = rtc::read_rtc();
    format!("{:02}:{:02}:{:02} {:04}-{:02}-{:02}", hour, minute, second, year as u64 + 2000, month, day)
}

// Calculate Unix time (seconds since 1970-01-01 00:00:00 UTC)
pub fn get_unix_time() -> u64 {
    let (second, minute, hour, day, month, year) = rtc::read_rtc();
    let year = year as i64 + 2000;
    let month = month as i64;
    let day = day as i64;
    let hour = hour as u64;
    let minute = minute as u64;
    let second = second as u64;

    // Calculate days since 1970-01-01
    let days = (year - 1970) * 365
        + (year - 1969) / 4
        - (year - 1901) / 100
        + (year - 1601) / 400
        + (367 * month - 362) / 12
        + if month <= 2 { 0 } else if is_leap_year(year) { -1 } else { -2 }
        + day - 1;

    // Calculate total seconds
    days as u64 * 24 * 3600 + hour * 3600 + minute * 60 + second
}

// Check if a year is a leap year
fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0) && (year % 100 != 0 || year % 400 == 0)
}

pub use pit::{
    sleep,
    get_unix_time_ns,
    wait
};