use chrono::{DateTime, FixedOffset, Local, TimeZone};

pub const PAGE_SIZE: usize = 50;

pub fn format_created(created: i64) -> String {
    match Local.timestamp_opt(created, 0).single() {
        Some(time) => time.format("%Y-%m-%d %H:%M").to_string(),
        None => String::new(),
    }
}

pub fn format_created_at(created: i64, offset: FixedOffset) -> String {
    DateTime::from_timestamp(created, 0)
        .map(|time| {
            time.with_timezone(&offset)
                .format("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_default()
}
