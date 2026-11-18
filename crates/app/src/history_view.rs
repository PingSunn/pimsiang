use std::sync::Arc;

use std::ops::Range;

use chrono::{DateTime, Datelike, FixedOffset, Local, NaiveDate, TimeZone};

use store::{Entry, HistoryService, Kind};

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

pub fn recorder(
    service: Arc<HistoryService>,
    model_id: Option<String>,
    on_recorded: impl Fn() + Send + Sync + 'static,
) -> impl FnMut(&str, Kind) + Send + 'static {
    let on_recorded = Arc::new(on_recorded);
    move |text, kind| {
        service.add(text.to_owned(), kind, model_id.clone(), |error| {
            tracing::warn!(%error, "cannot record the Final text in History");
        });
        let on_recorded = on_recorded.clone();
        service.run(move |_| on_recorded());
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page {
    pub entries: Vec<Entry>,
    pub matched: usize,
    pub total: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Day {
    Today,
    Yesterday,
    On(NaiveDate),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group<'a> {
    pub day: Day,
    pub entries: &'a [Entry],
}

fn date_of<Tz: TimeZone>(created: i64, zone: &Tz) -> Option<NaiveDate> {
    DateTime::from_timestamp(created, 0).map(|time| time.with_timezone(zone).date_naive())
}

pub fn day_of<Tz: TimeZone>(created: i64, now: i64, zone: Tz) -> Option<Day> {
    let date = date_of(created, &zone)?;
    let today = date_of(now, &zone)?;
    Some(if date == today {
        Day::Today
    } else if today.pred_opt() == Some(date) {
        Day::Yesterday
    } else {
        Day::On(date)
    })
}

pub fn group_by_day<Tz: TimeZone>(entries: &[Entry], now: i64, zone: Tz) -> Vec<Group<'_>> {
    let mut groups: Vec<Group<'_>> = Vec::new();
    let mut start = 0;
    let mut current: Option<Day> = None;
    for (index, entry) in entries.iter().enumerate() {
        let day = day_of(entry.created, now, zone.clone());
        if index > 0 && day != current {
            if let Some(day) = current {
                groups.push(Group {
                    day,
                    entries: &entries[start..index],
                });
            }
            start = index;
        }
        current = day;
    }
    if let Some(day) = current {
        groups.push(Group {
            day,
            entries: &entries[start..],
        });
    }
    groups
}

pub fn year_of<Tz: TimeZone>(now: i64, zone: Tz) -> i32 {
    date_of(now, &zone).map_or(1970, |date| date.year())
}

pub fn format_time<Tz: TimeZone>(created: i64, zone: Tz) -> String
where
    Tz::Offset: std::fmt::Display,
{
    DateTime::from_timestamp(created, 0)
        .map(|time| time.with_timezone(&zone).format("%H:%M").to_string())
        .unwrap_or_default()
}

fn is_thai_combining(character: char) -> bool {
    matches!(character, '\u{0E31}' | '\u{0E34}'..='\u{0E3A}' | '\u{0E47}'..='\u{0E4E}')
}

fn fold(character: char) -> char {
    character.to_lowercase().next().unwrap_or(character)
}

pub fn match_ranges(text: &str, query: &str) -> Vec<Range<usize>> {
    let needle: Vec<char> = query.trim().chars().map(fold).collect();
    if needle.is_empty() {
        return Vec::new();
    }
    let hay: Vec<(usize, char)> = text.char_indices().collect();
    let mut ranges: Vec<Range<usize>> = Vec::new();
    let mut index = 0;
    while index + needle.len() <= hay.len() {
        let hit = needle
            .iter()
            .enumerate()
            .all(|(offset, wanted)| fold(hay[index + offset].1) == *wanted);
        if !hit {
            index += 1;
            continue;
        }
        let mut first = index;
        while first > 0 && is_thai_combining(hay[first].1) {
            first -= 1;
        }
        let mut last = index + needle.len();
        while last < hay.len() && is_thai_combining(hay[last].1) {
            last += 1;
        }
        let end = hay.get(last).map_or(text.len(), |(byte, _)| *byte);
        let range = hay[first].0..end;
        match ranges.last_mut() {
            Some(previous) if previous.end >= range.start => {
                previous.end = previous.end.max(range.end);
            }
            _ => ranges.push(range),
        }
        index = last.max(index + needle.len());
    }
    ranges
}

pub fn thai_month_abbreviation(month: u32) -> &'static str {
    const MONTHS: [&str; 12] = [
        "ม.ค.",
        "ก.พ.",
        "มี.ค.",
        "เม.ย.",
        "พ.ค.",
        "มิ.ย.",
        "ก.ค.",
        "ส.ค.",
        "ก.ย.",
        "ต.ค.",
        "พ.ย.",
        "ธ.ค.",
    ];
    MONTHS[(month.clamp(1, 12) - 1) as usize]
}
