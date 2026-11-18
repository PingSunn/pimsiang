use rusqlite::Connection;
use store::{History, HistoryError, HistoryService, Kind, SCHEMA_VERSION};

fn open() -> (tempfile::TempDir, History) {
    let dir = tempfile::tempdir().unwrap();
    let history = History::open(&dir.path().join("nested/history.sqlite")).unwrap();
    (dir, history)
}

fn texts(entries: &[store::Entry]) -> Vec<&str> {
    entries.iter().map(|entry| entry.text.as_str()).collect()
}

#[test]
fn an_added_entry_is_listed_with_its_time_and_kind() {
    let (_dir, history) = open();

    history.add("สวัสดีครับ", 1_000, Kind::Pasted, None).unwrap();
    history
        .add("จดไว้", 2_000, Kind::Note, Some("int8-9f142a43"))
        .unwrap();

    let entries = history.list(0, 10).unwrap();
    assert_eq!(texts(&entries), ["จดไว้", "สวัสดีครับ"]);
    assert_eq!(entries[0].kind, Kind::Note);
    assert_eq!(entries[0].created, 2_000);
    assert_eq!(entries[0].model_id.as_deref(), Some("int8-9f142a43"));
    assert_eq!(entries[1].kind, Kind::Pasted);
    assert_eq!(entries[1].model_id, None);
}

#[test]
fn entries_come_newest_first_and_ties_keep_insertion_order() {
    let (_dir, history) = open();

    history.add("b", 20, Kind::Pasted, None).unwrap();
    history.add("a", 10, Kind::Pasted, None).unwrap();
    history.add("c1", 30, Kind::Pasted, None).unwrap();
    history.add("c2", 30, Kind::Pasted, None).unwrap();

    assert_eq!(texts(&history.list(0, 10).unwrap()), ["c2", "c1", "b", "a"]);
}

#[test]
fn paging_walks_the_whole_list_without_overlap() {
    let (_dir, history) = open();
    for index in 0..5 {
        history
            .add(&format!("entry {index}"), index, Kind::Pasted, None)
            .unwrap();
    }

    assert_eq!(texts(&history.list(0, 2).unwrap()), ["entry 4", "entry 3"]);
    assert_eq!(texts(&history.list(2, 2).unwrap()), ["entry 2", "entry 1"]);
    assert_eq!(texts(&history.list(4, 2).unwrap()), ["entry 0"]);
    assert!(history.list(6, 2).unwrap().is_empty());
}

#[test]
fn thai_substrings_are_found_inside_words_without_spaces() {
    let (_dir, history) = open();
    history
        .add("วันนี้อากาศดีมากเลยครับ", 1, Kind::Pasted, None)
        .unwrap();
    history
        .add("พรุ่งนี้ไปประชุมที่ออฟฟิศ", 2, Kind::Note, None)
        .unwrap();

    assert_eq!(
        texts(&history.search("อากาศ", 0, 10).unwrap()),
        ["วันนี้อากาศดีมากเลยครับ"]
    );
    assert_eq!(
        texts(&history.search("ประชุมที่ออฟ", 0, 10).unwrap()),
        ["พรุ่งนี้ไปประชุมที่ออฟฟิศ"]
    );
    assert!(history.search("ฝนตก", 0, 10).unwrap().is_empty());
}

#[test]
fn search_is_newest_first_pages_and_ignores_surrounding_spaces() {
    let (_dir, history) = open();
    history.add("ประชุมทีม หนึ่ง", 1, Kind::Pasted, None).unwrap();
    history.add("ประชุมทีม สอง", 2, Kind::Pasted, None).unwrap();
    history.add("ประชุมทีม สาม", 3, Kind::Pasted, None).unwrap();

    assert_eq!(
        texts(&history.search("  ประชุมทีม ", 0, 2).unwrap()),
        ["ประชุมทีม สาม", "ประชุมทีม สอง"]
    );
    assert_eq!(
        texts(&history.search("ประชุมทีม", 2, 2).unwrap()),
        ["ประชุมทีม หนึ่ง"]
    );
}

#[test]
fn latin_search_ignores_case() {
    let (_dir, history) = open();
    history
        .add("Deploy the Kubernetes cluster", 1, Kind::Pasted, None)
        .unwrap();

    assert_eq!(history.search("kubernetes", 0, 10).unwrap().len(), 1);
}

#[test]
fn queries_shorter_than_three_characters_fall_back_to_a_substring_match() {
    let (_dir, history) = open();
    history.add("ไปกินข้าวกัน", 1, Kind::Pasted, None).unwrap();
    history.add("ไปทำงาน", 2, Kind::Pasted, None).unwrap();
    history.add("100% done", 3, Kind::Pasted, None).unwrap();
    history.add("a_b", 4, Kind::Pasted, None).unwrap();
    history.add("axb", 5, Kind::Pasted, None).unwrap();

    assert_eq!(
        texts(&history.search("ไป", 0, 10).unwrap()),
        ["ไปทำงาน", "ไปกินข้าวกัน"]
    );
    assert_eq!(texts(&history.search("ข", 0, 10).unwrap()), ["ไปกินข้าวกัน"]);
    assert_eq!(texts(&history.search("%", 0, 10).unwrap()), ["100% done"]);
    assert_eq!(texts(&history.search("_", 0, 10).unwrap()), ["a_b"]);
}

#[test]
fn quotes_and_fts_operators_in_a_query_are_plain_text() {
    let (_dir, history) = open();
    history
        .add("เขาพูดว่า \"ตกลง\" แล้ว", 1, Kind::Pasted, None)
        .unwrap();
    history.add("alpha beta", 2, Kind::Pasted, None).unwrap();

    assert_eq!(history.search("\"ตกลง\"", 0, 10).unwrap().len(), 1);
    assert!(history.search("alpha OR zzz", 0, 10).unwrap().is_empty());
    assert!(history.search("alp*", 0, 10).unwrap().is_empty());
    assert_eq!(history.search("   ", 0, 10).unwrap().len(), 2);
}

#[test]
fn deleting_one_entry_removes_it_from_the_list_and_from_search() {
    let (_dir, history) = open();
    let keep = history.add("เก็บไว้ก่อนนะ", 1, Kind::Pasted, None).unwrap();
    let drop = history.add("ลบอันนี้ทิ้ง", 2, Kind::Pasted, None).unwrap();

    assert!(history.delete(drop).unwrap());
    assert!(!history.delete(drop).unwrap());

    let entries = history.list(0, 10).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, keep);
    assert!(history.search("ลบอันนี้", 0, 10).unwrap().is_empty());
    assert_eq!(history.search("เก็บไว้", 0, 10).unwrap().len(), 1);
}

#[test]
fn clearing_removes_everything_and_the_store_keeps_working() {
    let (_dir, history) = open();
    history.add("หนึ่งสองสาม", 1, Kind::Pasted, None).unwrap();
    history.add("สี่ห้าหก", 2, Kind::Note, None).unwrap();

    history.clear().unwrap();

    assert!(history.list(0, 10).unwrap().is_empty());
    assert!(history.search("สองสาม", 0, 10).unwrap().is_empty());
    assert_eq!(history.latest().unwrap(), None);
    history.add("เจ็ดแปดเก้า", 3, Kind::Pasted, None).unwrap();
    assert_eq!(history.search("แปดเก้า", 0, 10).unwrap().len(), 1);
}

#[test]
fn latest_is_the_newest_entry_or_none() {
    let (_dir, history) = open();
    assert_eq!(history.latest().unwrap(), None);

    history.add("เก่า", 1, Kind::Pasted, None).unwrap();
    history.add("ใหม่", 2, Kind::Note, None).unwrap();

    assert_eq!(history.latest().unwrap().unwrap().text, "ใหม่");
}

#[test]
fn entries_survive_closing_and_reopening() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    History::open(&path)
        .unwrap()
        .add("จำไว้นะ", 1, Kind::Pasted, None)
        .unwrap();

    let history = History::open(&path).unwrap();

    assert_eq!(history.version().unwrap(), SCHEMA_VERSION);
    assert_eq!(texts(&history.list(0, 10).unwrap()), ["จำไว้นะ"]);
}

#[test]
fn a_database_without_a_schema_version_is_migrated_forward() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    Connection::open(&path).unwrap();

    let history = History::open(&path).unwrap();

    assert_eq!(history.version().unwrap(), SCHEMA_VERSION);
    history
        .add("ทดสอบ migrate", 1, Kind::Pasted, Some("m"))
        .unwrap();
    assert_eq!(history.search("migrate", 0, 10).unwrap().len(), 1);
}

#[test]
fn a_version_one_database_keeps_its_entries_and_gains_the_model_column() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let old = Connection::open(&path).unwrap();
    old.execute_batch(
        "CREATE TABLE entries (id INTEGER PRIMARY KEY, text TEXT NOT NULL, created INTEGER NOT NULL, kind TEXT NOT NULL);
         CREATE INDEX entries_created ON entries (created DESC, id DESC);
         CREATE VIRTUAL TABLE entries_fts USING fts5(text, content = 'entries', content_rowid = 'id', tokenize = 'trigram');
         CREATE TRIGGER entries_after_insert AFTER INSERT ON entries BEGIN
             INSERT INTO entries_fts (rowid, text) VALUES (new.id, new.text);
         END;
         CREATE TRIGGER entries_after_delete AFTER DELETE ON entries BEGIN
             INSERT INTO entries_fts (entries_fts, rowid, text) VALUES ('delete', old.id, old.text);
         END;
         INSERT INTO entries (text, created, kind) VALUES ('ข้อความเก่าจากรุ่นแรก', 5, 'note');
         PRAGMA user_version = 1;",
    )
    .unwrap();
    drop(old);

    let history = History::open(&path).unwrap();

    assert_eq!(history.version().unwrap(), SCHEMA_VERSION);
    let found = history.search("เก่าจาก", 0, 10).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, Kind::Note);
    assert_eq!(found[0].model_id, None);
}

#[test]
fn a_database_from_a_newer_app_is_refused_and_left_alone() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let newer = Connection::open(&path).unwrap();
    newer
        .pragma_update(None, "user_version", SCHEMA_VERSION + 1)
        .unwrap();
    drop(newer);

    let error = History::open(&path).err().unwrap();

    assert!(matches!(error, HistoryError::TooNew { .. }));
}

#[test]
fn the_service_stores_entries_in_order_on_its_own_thread() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let service = HistoryService::spawn(&path).unwrap();

    service.add("หนึ่ง".to_owned(), Kind::Pasted, Some("m".to_owned()), |_| {});
    service.add("สอง".to_owned(), Kind::Note, None, |_| {});
    service.flush();

    let entries = History::open(&path).unwrap().list(0, 10).unwrap();
    assert_eq!(texts(&entries), ["สอง", "หนึ่ง"]);
    assert_eq!(entries[1].model_id.as_deref(), Some("m"));
}

#[test]
fn the_service_answers_queries_through_a_callback() {
    let dir = tempfile::tempdir().unwrap();
    let service = HistoryService::spawn(&dir.path().join("history.sqlite")).unwrap();
    service.add("ค้นหาให้เจอ".to_owned(), Kind::Pasted, None, |_| {});
    let (reply, answer) = std::sync::mpsc::channel();

    service.run(move |history| {
        let _ = reply.send(history.search("หาให้", 0, 10).map(|found| found.len()).ok());
    });

    assert_eq!(answer.recv().unwrap(), Some(1));
}
