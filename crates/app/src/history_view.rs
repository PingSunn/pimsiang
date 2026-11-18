use std::sync::Arc;

use chrono::{DateTime, FixedOffset, Local, TimeZone};

use store::{HistoryService, Kind};

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
