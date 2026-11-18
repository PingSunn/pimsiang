use platform::{
    Permission, PermissionReport, PermissionStatus, PillContent, PillError, PillNotice, PillState,
    Status, TriggerKey,
};
use store::Kind;

macro_rules! th {
    ($first:literal $(| $rest:literal)*) => {
        concat!($first $(, "\u{200a}", $rest)*)
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    English,
    Thai,
}

impl Language {
    pub fn from_tags<S: AsRef<str>>(tags: &[S]) -> Self {
        match tags.first() {
            Some(tag) if tag.as_ref().to_ascii_lowercase().starts_with("th") => Self::Thai,
            _ => Self::English,
        }
    }

    fn pick(self, english: &'static str, thai: &'static str) -> &'static str {
        match self {
            Self::English => english,
            Self::Thai => thai,
        }
    }

    pub fn window_title(self) -> &'static str {
        self.pick("PimSiang setup", "ตั้งค่า PimSiang")
    }

    pub fn heading(self) -> &'static str {
        self.pick("Let's get PimSiang ready", "มาเริ่มใช้พิมพ์เสียงกัน")
    }

    pub fn intro_left(self, left: usize) -> String {
        match (self, left) {
            (Self::English, 0) => "Everything is ready. This window closes by itself.".to_owned(),
            (Self::English, 1) => {
                "One step left, then you can talk. This window closes by itself when you are done."
                    .to_owned()
            }
            (Self::English, n) => format!(
                "{n} steps left, then you can talk. This window closes by itself when you are done."
            ),
            (Self::Thai, 0) => th!("พร้อม"
                | "ใช้งาน"
                | "แล้ว หน้าต่าง"
                | "นี้"
                | "จะ"
                | "ปิดเอง")
            .to_owned(),
            (Self::Thai, n) => format!(
                th!("อีก {n} ขั้นตอน"
                    | "ก็"
                    | "พูด"
                    | "ได้เลย หน้าต่าง"
                    | "นี้"
                    | "จะ"
                    | "ปิดเอง"
                    | "เมื่อ"
                    | "ครบ"),
                n = n
            ),
        }
    }

    pub fn setup_bubble(self) -> &'static str {
        self.pick("Hello!", "สวัสดีครับ")
    }

    pub fn setup_hint(self) -> &'static str {
        self.pick(
            "Hold and speak. Let go and the text lands at the cursor.",
            th!("กดค้าง"
                | "แล้ว"
                | "พูด ปล่อย"
                | "แล้ว"
                | "ข้อความ"
                | "จะ"
                | "ไป"
                | "อยู่"
                | "ตรง"
                | "เคอร์เซอร์"),
        )
    }

    pub fn terminal_note(self) -> &'static str {
        self.pick(
            "PimSiang is running from a terminal, so these permissions belong to the terminal app that started it. Grant them to that app.",
            th!("PimSiang กำลัง" | "ทำงาน" | "ผ่าน" | "เทอร์มินัล สิทธิ์" | "เหล่านี้" | "จึง" | "เป็นของ" | "แอป" | "เทอร์มินัล" | "ที่" | "เปิด" | "มัน ให้" | "อนุญาต" | "กับ" | "แอป" | "นั้น"),
        )
    }

    pub fn menu_setup(self) -> &'static str {
        self.pick("Set up permissions...", "ตั้งค่าสิทธิ์...")
    }

    pub fn menu_all_granted(self) -> &'static str {
        self.pick("All permissions granted", "ได้รับสิทธิ์ครบแล้ว")
    }

    pub fn menu_microphone(self) -> &'static str {
        self.pick("Microphone", "ไมโครโฟน")
    }

    pub fn menu_system_default(self) -> &'static str {
        self.pick("System default", "ค่าเริ่มต้นของระบบ")
    }

    pub fn menu_update(self, version: &str) -> String {
        format!(
            "{}{version}{}",
            self.pick("Update available: v", "มีเวอร์ชันใหม่: v"),
            self.pick(" — run brew upgrade", " — รัน brew upgrade"),
        )
    }

    pub fn menu_update_copied(self) -> &'static str {
        self.pick("Copied brew upgrade command", "คัดลอกคำสั่ง brew upgrade แล้ว")
    }

    pub fn menu_dismiss_update(self) -> &'static str {
        self.pick("Dismiss this update notice", "ซ่อนการแจ้งเวอร์ชันใหม่นี้")
    }

    pub fn menu_missing(self, report: &PermissionReport) -> String {
        let names: Vec<&str> = report
            .missing()
            .into_iter()
            .map(|permission| self.permission_name(permission))
            .collect();
        format!(
            "{}{}",
            self.pick("Missing permissions: ", "ยังไม่ได้ให้สิทธิ์: "),
            names.join(", ")
        )
    }

    pub fn permission_name(self, permission: Permission) -> &'static str {
        match permission {
            Permission::Microphone => self.pick("Microphone", "ไมโครโฟน"),
            Permission::Accessibility => self.pick("Accessibility", "การช่วยการเข้าถึง"),
            Permission::InputMonitoring => self.pick("Input Monitoring", "การตรวจสอบอินพุต"),
        }
    }

    pub fn permission_why(self, permission: Permission, key: TriggerKey) -> String {
        let key = key.label();
        match (permission, self) {
            (Permission::Microphone, Self::English) => {
                format!("Hears you while you hold {key}. Audio never leaves this Mac.")
            }
            (Permission::Microphone, Self::Thai) => {
                format!(
                    th!("ใช้"
                        | "ฟัง"
                        | "เสียง"
                        | "ของ"
                        | "คุณ"
                        | "ขณะ"
                        | "กด {key} ค้างไว้ เสียง"
                        | "จะ"
                        | "ไม่"
                        | "ถูก"
                        | "ส่งออก"
                        | "จาก"
                        | "เครื่อง"
                        | "นี้"),
                    key = key
                )
            }
            (Permission::Accessibility, _) => self
                .pick(
                    "Pastes the transcribed text into the app you are typing in.",
                    th!("ใช้"
                        | "วาง"
                        | "ข้อความ"
                        | "ที่"
                        | "ถอดเสียง"
                        | "ได้"
                        | "ลง"
                        | "ใน"
                        | "แอป"
                        | "ที่"
                        | "คุณ"
                        | "กำลัง"
                        | "พิมพ์"),
                )
                .to_owned(),
            (Permission::InputMonitoring, Self::English) => {
                format!("Notices when you hold {key} so PimSiang knows when to listen.")
            }
            (Permission::InputMonitoring, Self::Thai) => {
                format!(
                    th!("ใช้"
                        | "ตรวจ"
                        | "ว่า"
                        | "คุณ"
                        | "กด {key} ค้างไว้ เพื่อให้ PimSiang รู้"
                        | "ว่า"
                        | "ต้อง"
                        | "เริ่ม"
                        | "ฟัง"
                        | "เมื่อไร"),
                    key = key
                )
            }
        }
    }

    pub fn status_label(self, status: PermissionStatus) -> &'static str {
        match status {
            PermissionStatus::Granted => self.pick("Granted", "อนุญาตแล้ว"),
            PermissionStatus::NotDetermined => self.pick("Not granted yet", "ยังไม่ได้อนุญาต"),
            PermissionStatus::Denied => self.pick("Not granted", "ยังไม่ได้รับอนุญาต"),
        }
    }

    pub fn allow_microphone(self) -> &'static str {
        self.pick("Allow microphone", "อนุญาตไมโครโฟน")
    }

    pub fn open_settings(self) -> &'static str {
        self.pick("Open System Settings", "เปิดการตั้งค่าระบบ")
    }

    pub fn model_title(self) -> &'static str {
        self.pick("Speech model", "โมเดลถอดเสียง")
    }

    pub fn model_explain(self, bytes: u64) -> String {
        let size = download_size(bytes);
        match self {
            Self::English => format!(
                "One-time download, about {size}. If it drops, it resumes where it stopped."
            ),
            Self::Thai => format!(
                th!("ดาวน์โหลด"
                    | "ครั้งเดียว ราว {size} หลุด"
                    | "แล้ว"
                    | "ต่อ"
                    | "จาก"
                    | "เดิม"
                    | "ได้"),
                size = size
            ),
        }
    }

    pub fn settings_precision(self) -> &'static str {
        self.pick("Precision", "ความแม่นยำ")
    }

    pub fn settings_precision_name(self, precision: store::Precision) -> &'static str {
        match precision {
            store::Precision::Accurate => self.pick("Accurate", "แม่นยำ"),
            store::Precision::Light => self.pick("Light", "เบา"),
        }
    }

    pub fn settings_precision_hint(self, precision: store::Precision, bytes: u64) -> String {
        let size = download_size(bytes);
        match (self, precision) {
            (Self::English, store::Precision::Accurate) => {
                format!("Best accuracy. Downloads about {size}.")
            }
            (Self::English, store::Precision::Light) => {
                format!("Downloads about {size} and uses less memory.")
            }
            (Self::Thai, store::Precision::Accurate) => {
                format!(th!("ถูกต้อง" | "ที่สุด · ดาวน์โหลด" | "ราว {size}"), size = size)
            }
            (Self::Thai, store::Precision::Light) => format!(
                th!("ดาวน์โหลด" | "ราว {size} และ" | "ใช้" | "หน่วยความจำ" | "น้อยกว่า"),
                size = size
            ),
        }
    }

    pub fn settings_precision_help(self) -> &'static str {
        self.pick(
            "Changing it downloads the other build in the background. The new one starts with your next Utterance once it is checked.",
            th!("เมื่อ"
                | "เปลี่ยน"
                | "ระบบ"
                | "จะ"
                | "ดาวน์โหลด"
                | "ตัว"
                | "ใหม่"
                | "ในพื้นหลัง"
                | "และ"
                | "เริ่ม"
                | "ใช้"
                | "กับ"
                | "การ"
                | "พูด"
                | "ครั้ง"
                | "ถัดไป"
                | "หลัง"
                | "ตรวจสอบ"
                | "เสร็จ"),
        )
    }

    pub fn settings_precision_keeps_working(self) -> &'static str {
        self.pick(
            "The current model keeps working meanwhile.",
            th!("ระหว่าง"
                | "นี้"
                | "โมเดล"
                | "เดิม"
                | "ยัง"
                | "ใช้งาน"
                | "ได้"
                | "ตามปกติ"),
        )
    }

    pub fn model_downloading(self, done: u64, total: u64) -> String {
        let (done, total) = (
            group_thousands(done / 1_000_000),
            group_thousands(total / 1_000_000),
        );
        match self {
            Self::English => format!("{done} of {total} MB"),
            Self::Thai => format!("{done} จาก {total} MB"),
        }
    }

    pub fn model_verifying(self) -> &'static str {
        self.pick(
            "Checking the download...",
            th!("กำลัง" | "ตรวจสอบ" | "ไฟล์" | "ที่" | "ดาวน์โหลด..."),
        )
    }

    pub fn model_failed(self, reason: &str) -> String {
        format!(
            "{}{reason}",
            self.pick("Download failed: ", "ดาวน์โหลดไม่สำเร็จ: ")
        )
    }

    pub fn model_ready(self) -> &'static str {
        self.pick("Downloaded and checked", "ดาวน์โหลดและตรวจสอบแล้ว")
    }

    pub fn model_retry(self) -> &'static str {
        self.pick("Try again", "ลองอีกครั้ง")
    }

    pub fn menu_open_logs(self) -> &'static str {
        self.pick("Open log folder", "เปิดโฟลเดอร์ล็อก")
    }

    pub fn menu_quit(self) -> &'static str {
        self.pick("Quit PimSiang", "ออกจากพิมพ์เสียง")
    }

    pub fn tooltip(self, status: Status, permission_missing: bool) -> &'static str {
        if permission_missing {
            return self.pick("PimSiang: permission missing", "PimSiang: ยังไม่ได้ให้สิทธิ์");
        }
        match status {
            Status::Ready => self.pick("PimSiang: ready", "PimSiang: พร้อมใช้งาน"),
            Status::Listening => self.pick("PimSiang: listening", "PimSiang: กำลังฟัง"),
            Status::Transcribing => self.pick("PimSiang: transcribing", "PimSiang: กำลังถอดเสียง"),
        }
    }

    pub fn pill_listening(self) -> &'static str {
        self.pick("Listening…", "กำลังฟัง…")
    }

    pub fn pill_loading_model(self) -> &'static str {
        self.pick("Loading the Model…", "กำลังโหลดโมเดล…")
    }

    pub fn pill_transcribing(self) -> &'static str {
        self.pick("Transcribing…", "กำลังเขียนให้…")
    }

    pub fn pill_error(self, error: PillError) -> String {
        match error {
            PillError::PermissionMissing(permission) => {
                let name = self.permission_name(permission);
                match self {
                    Self::English => format!("{name} permission is missing"),
                    Self::Thai => format!("ยังไม่ได้ให้สิทธิ์ {name}"),
                }
            }
            PillError::ModelNotLoaded => {
                self.pick("Model not loaded", "ยังไม่ได้โหลดโมเดล").to_owned()
            }
            PillError::ModelNotReady => self.pick("Model not ready", "โมเดลยังไม่พร้อม").to_owned(),
            PillError::MicrophoneUnavailable => self
                .pick("No microphone available", "ไม่พบไมโครโฟน")
                .to_owned(),
            PillError::TranscriptionFailed => self
                .pick("Transcription failed", "ถอดเสียงไม่สำเร็จ")
                .to_owned(),
            PillError::PasteFailed => self
                .pick("Could not paste the text", "วางข้อความไม่ได้")
                .to_owned(),
        }
    }

    pub fn pill_notice(self, notice: PillNotice) -> &'static str {
        match notice {
            PillNotice::MicrophoneFellBack => self.pick(
                "Microphone not found, using the system default",
                "ไม่พบไมโครโฟน ใช้ค่าเริ่มต้นของระบบแทน",
            ),
            PillNotice::MicrophoneLost => {
                self.pick("Microphone disconnected", "ไมโครโฟนขาดการเชื่อมต่อ")
            }
            PillNotice::NoTranscriptYet => self.pick("No transcript yet", "ยังไม่มีข้อความ"),
            PillNotice::QuickNoteSaved => self.pick("Saved as a Note", "จดเป็นโน้ตแล้ว"),
            PillNotice::NoTextFieldKeptAsNote => self.pick(
                "No text field focused. Saved as a Note and copied",
                "ไม่มีช่องข้อความ บันทึกเป็นโน้ตและคัดลอกแล้ว",
            ),
            PillNotice::PasteFailedKeptAsNote => self.pick(
                "Could not paste. Saved as a Note and copied",
                "วางไม่ได้ บันทึกเป็นโน้ตและคัดลอกแล้ว",
            ),
        }
    }

    pub fn pill_text(self, state: &PillState) -> Option<String> {
        match state {
            PillState::Hidden => None,
            PillState::Listening => Some(self.pill_listening().to_owned()),
            PillState::LoadingModel => Some(self.pill_loading_model().to_owned()),
            PillState::Partial(partial) => Some(partial.text.clone()),
            PillState::Transcribing => Some(self.pill_transcribing().to_owned()),
            PillState::Notice(notice) => Some(self.pill_notice(*notice).to_owned()),
            PillState::Error(error) => Some(self.pill_error(*error)),
        }
    }

    pub fn pill_badge_note(self) -> &'static str {
        self.pick("Note", "โน้ต")
    }

    pub fn pill_content(self, state: &PillState) -> Option<PillContent> {
        let text = self.pill_text(state)?;
        let plain = |text: String| PillContent {
            settled: text,
            ..PillContent::default()
        };
        Some(match state {
            PillState::Partial(partial) => {
                let (settled, tentative) = partial.split();
                PillContent {
                    settled: settled.to_owned(),
                    tentative: tentative.to_owned(),
                    caret: true,
                    ..PillContent::default()
                }
            }
            PillState::Transcribing => PillContent {
                dots: true,
                ..plain(text)
            },
            PillState::Notice(
                PillNotice::QuickNoteSaved
                | PillNotice::NoTextFieldKeptAsNote
                | PillNotice::PasteFailedKeptAsNote,
            ) => PillContent {
                badge: Some(self.pill_badge_note().to_owned()),
                ..plain(text)
            },
            _ => plain(text),
        })
    }

    pub fn trigger_key_symbol(key: TriggerKey) -> &'static str {
        match key {
            TriggerKey::RightOption => "⌥",
            TriggerKey::RightCommand => "⌘",
            TriggerKey::RightControl => "⌃",
        }
    }

    pub fn menu_header(
        self,
        status: Status,
        missing: Option<&PermissionReport>,
        key: TriggerKey,
    ) -> (String, String) {
        let key_name = format!("{} {}", Self::trigger_key_symbol(key), key.label());
        if let Some(report) = missing {
            let names: Vec<&str> = report
                .missing()
                .into_iter()
                .map(|permission| self.permission_name(permission))
                .collect();
            return (
                self.pick("Permission missing", "ยังไม่ได้ให้สิทธิ์").to_owned(),
                names.join(", "),
            );
        }
        match status {
            Status::Ready => (
                self.pick("Ready to listen", "พร้อมฟังแล้ว").to_owned(),
                match self {
                    Self::English => format!("Hold {key_name} and speak"),
                    Self::Thai => format!("กด {key_name} ค้างแล้วพูด"),
                },
            ),
            Status::Listening => (
                self.pick("Listening", "กำลังฟัง").to_owned(),
                self.pick("Release the key when you finish", "ปล่อยปุ่มเมื่อพูดจบ")
                    .to_owned(),
            ),
            Status::Transcribing => (
                self.pick("Writing it up…", "กำลังเขียนให้…").to_owned(),
                self.pick(
                    "The text will land at the cursor",
                    "ข้อความจะไปอยู่ตรงเคอร์เซอร์",
                )
                .to_owned(),
            ),
        }
    }

    pub fn menu_history(self) -> &'static str {
        self.pick("History…", "ประวัติ…")
    }

    pub fn menu_paste_last(self) -> &'static str {
        self.pick("Paste last transcript", "วางข้อความล่าสุด")
    }

    pub fn menu_settings(self) -> &'static str {
        self.pick("Settings…", "ตั้งค่า…")
    }

    pub fn settings_title(self) -> &'static str {
        self.pick("PimSiang settings", "การตั้งค่า PimSiang")
    }

    pub fn settings_applies_now(self) -> &'static str {
        self.pick("Changes apply straight away.", "เปลี่ยนแล้วมีผลทันที")
    }

    pub fn section_general(self) -> &'static str {
        self.pick("General", "ทั่วไป")
    }

    pub fn section_trigger(self) -> &'static str {
        self.pick("Trigger", "ปุ่มสั่งพูด")
    }

    pub fn section_sounds_clipboard(self) -> &'static str {
        self.pick("Clipboard & Sounds", "คลิปบอร์ดและเสียง")
    }

    pub fn history_heading(self) -> &'static str {
        self.pick("History", "ประวัติ")
    }

    pub fn setup_progress(self, done: usize, total: usize) -> String {
        match self {
            Self::English => format!("{done} of {total} ready"),
            Self::Thai => format!("พร้อมแล้ว {done} จาก {total}"),
        }
    }

    pub fn settings_trigger_key(self) -> &'static str {
        self.pick("Trigger key", "ปุ่มสั่งพูด")
    }

    pub fn settings_trigger_key_help(self) -> &'static str {
        self.pick(
            "One key does three things. Pick one that does not clash with your work.",
            th!("ปุ่ม"
                | "เดียว"
                | "ทำได้"
                | "สามอย่าง เลือก"
                | "ปุ่ม"
                | "ที่"
                | "ไม่ชน"
                | "กับ"
                | "งาน"
                | "ของ"
                | "คุณ"),
        )
    }

    pub fn settings_language_mode(self) -> &'static str {
        self.pick("Spoken language", "ภาษาที่พูด")
    }

    pub fn settings_language_mix(self) -> &'static str {
        self.pick("Thai mixed with English", "ไทยปนอังกฤษ")
    }

    pub fn settings_language_mix_example(self) -> [(&'static str, bool); 5] {
        [
            ("ขอ ", false),
            ("deploy", true),
            (" หลัง ", false),
            ("review", true),
            (" เสร็จ", false),
        ]
    }

    pub fn settings_language_thai(self) -> &'static str {
        self.pick("Thai only", "ไทยล้วน")
    }

    pub fn settings_language_thai_example(self) -> [(&'static str, bool); 5] {
        [
            ("ขอ ", false),
            ("ดีพลอย", true),
            (" หลัง ", false),
            ("รีวิว", true),
            (" เสร็จ", false),
        ]
    }

    pub fn settings_language_help(self) -> &'static str {
        self.pick(
            "Takes effect on your next Utterance.",
            th!("ใช้" | "กับ" | "การ" | "พูด" | "ครั้ง" | "ถัดไป"),
        )
    }

    pub fn settings_sounds(self) -> &'static str {
        self.pick(
            "Play a sound when listening starts and stops",
            th!("เล่น"
                | "เสียง"
                | "เมื่อ"
                | "เริ่ม"
                | "และ"
                | "หยุด"
                | "ฟัง"),
        )
    }

    pub fn settings_clipboard_restore(self) -> &'static str {
        self.pick(
            "Restore the clipboard after pasting",
            th!("คืน"
                | "ค่า"
                | "คลิปบอร์ด"
                | "เดิม"
                | "หลัง"
                | "วาง"
                | "ข้อความ"),
        )
    }

    pub fn settings_launch_at_login(self) -> &'static str {
        self.pick(
            "Open PimSiang at login",
            th!("เปิด PimSiang เมื่อ" | "เข้าสู่" | "ระบบ"),
        )
    }

    pub fn settings_login_unbundled(self) -> &'static str {
        self.pick(
            "Only works from the installed PimSiang app, not when it runs from a terminal.",
            th!("ใช้ได้"
                | "เฉพาะ"
                | "แอป PimSiang ที่"
                | "ติดตั้ง"
                | "แล้ว ไม่"
                | "รองรับ"
                | "เมื่อ"
                | "รัน"
                | "จาก"
                | "เทอร์มินัล"),
        )
    }

    pub fn settings_login_needs_approval(self) -> &'static str {
        self.pick(
            "Allow PimSiang in System Settings > General > Login Items.",
            "อนุญาต PimSiang ที่ System Settings > General > Login Items",
        )
    }

    pub fn settings_login_open_settings(self) -> &'static str {
        self.pick("Open Login Items", "เปิด Login Items")
    }

    pub fn settings_login_failed(self, reason: &str) -> String {
        format!(
            "{}{reason}",
            self.pick(
                "Could not change the login item: ",
                th!("เปลี่ยน"
                    | "การ"
                    | "เปิด"
                    | "ตอน"
                    | "เข้าสู่"
                    | "ระบบ"
                    | "ไม่"
                    | "สำเร็จ: ")
            )
        )
    }

    pub fn history_title(self) -> &'static str {
        self.pick("PimSiang history", "ประวัติ PimSiang")
    }

    pub fn history_search_hint(self) -> &'static str {
        self.pick("Search what you said", "ค้นหาสิ่งที่เคยพูด")
    }

    pub fn history_empty(self) -> &'static str {
        self.pick(
            "Nothing here yet. Everything you dictate is kept here, on this Mac only.",
            th!("ยัง"
                | "ไม่มี"
                | "รายการ ทุก"
                | "อย่าง"
                | "ที่"
                | "คุณ"
                | "พูด"
                | "จะ"
                | "ถูก"
                | "เก็บ"
                | "ไว้"
                | "ที่นี่ ใน"
                | "เครื่อง"
                | "นี้"
                | "เท่านั้น"),
        )
    }

    pub fn history_no_match(self) -> &'static str {
        self.pick(
            "No entries match your search.",
            th!("ไม่พบ" | "รายการ" | "ที่" | "ตรง" | "กับ" | "คำค้นหา"),
        )
    }

    pub fn history_kind(self, kind: Kind) -> &'static str {
        match kind {
            Kind::Pasted => self.pick("Pasted", "วางแล้ว"),
            Kind::Note => self.pick("Note", "โน้ต"),
        }
    }

    pub fn history_copy(self) -> &'static str {
        self.pick("Copy", "คัดลอก")
    }

    pub fn history_copied(self) -> &'static str {
        self.pick("Copied", "คัดลอกแล้ว")
    }

    pub fn history_delete(self) -> &'static str {
        self.pick("Delete", "ลบ")
    }

    pub fn history_clear_all(self) -> &'static str {
        self.pick("Clear all", "ล้างทั้งหมด")
    }

    pub fn history_confirm_clear(self) -> &'static str {
        self.pick(
            "Delete all history? This cannot be undone.",
            th!("ลบ"
                | "ประวัติ"
                | "ทั้งหมด? การ"
                | "ลบ"
                | "นี้"
                | "ย้อนกลับ"
                | "ไม่ได้"),
        )
    }

    pub fn history_confirm_yes(self) -> &'static str {
        self.pick("Delete everything", "ลบทั้งหมดเลย")
    }

    pub fn history_cancel(self) -> &'static str {
        self.pick("Cancel", "ยกเลิก")
    }

    pub fn history_show_more(self) -> &'static str {
        self.pick("Show more", "แสดงเพิ่ม")
    }

    pub fn history_unavailable(self) -> &'static str {
        self.pick(
            "History is unavailable: the database could not be opened.",
            th!("ใช้งาน"
                | "ประวัติ"
                | "ไม่ได้: เปิด"
                | "ฐานข้อมูล"
                | "ไม่"
                | "สำเร็จ"),
        )
    }

    pub fn settings_pill_group(self) -> &'static str {
        self.pick("Pill position", "ตำแหน่ง Pill")
    }

    pub fn settings_pill_notch(self) -> &'static str {
        self.pick("Notch", "รอยบาก")
    }

    pub fn settings_pill_floating(self) -> &'static str {
        self.pick("Floating", "ลอย")
    }

    pub fn settings_pill_position(self) -> &'static str {
        self.pick("Position…", "จัดตำแหน่ง…")
    }

    pub fn settings_pill_done(self) -> &'static str {
        self.pick("Done", "เสร็จ")
    }

    pub fn settings_pill_notch_help(self) -> &'static str {
        self.pick(
            "The Pill comes out of the notch at the top centre, or sits below the menu bar when the screen has no notch.",
            th!("Pill"
                | "โผล่ออกมา"
                | "จากรอยบาก"
                | "ตรงกลาง"
                | "ด้านบน"
                | "หรืออยู่"
                | "ใต้แถบเมนู"
                | "ถ้าจอ"
                | "ไม่มีรอยบาก"),
        )
    }

    pub fn settings_pill_floating_help(self) -> &'static str {
        self.pick(
            "The Pill floats where you put it, on the screen your pointer is on.",
            th!("Pill"
                | "ลอยอยู่"
                | "ตรงที่"
                | "คุณวางไว้"
                | "บนจอ"
                | "ที่เมาส์"
                | "อยู่"),
        )
    }

    pub fn settings_pill_placing_help(self) -> &'static str {
        self.pick(
            "Drag the sample Pill on screen, then press Done or Esc.",
            th!("ลาก"
                | "ตัวอย่าง Pill"
                | "บนหน้าจอ"
                | "แล้วกด"
                | "เสร็จ"
                | "หรือ Esc"),
        )
    }

    pub fn pill_place_hint(self) -> &'static str {
        self.pick(
            "Drag it where you want it, then press Done / Esc",
            "ลากไปวางตรงที่ต้องการ แล้วกด เสร็จ / Esc",
        )
    }

    pub fn settings_system_group(self) -> &'static str {
        self.pick("System", "ระบบ")
    }

    pub fn settings_launch_at_login_help(self) -> &'static str {
        self.pick(
            "The bird icon waits in the menu bar every time you start your Mac.",
            th!("ไอคอน"
                | "นก"
                | "จะ"
                | "รอ"
                | "อยู่"
                | "บน"
                | "แถบเมนู"
                | "ทุกครั้ง"
                | "ที่"
                | "เปิดเครื่อง"),
        )
    }

    pub fn settings_unload_when_idle(self) -> &'static str {
        self.pick(
            "Rest the model after 10 idle minutes",
            th!("พัก" | "โมเดล" | "เมื่อ" | "ไม่ได้" | "ใช้ 10 นาที"),
        )
    }

    pub fn settings_unload_when_idle_help(self) -> &'static str {
        self.pick(
            "Gives back about 800 MB of memory. The next start is a little slower.",
            th!("คืน"
                | "หน่วยความจำ"
                | "ราว 800 MB ครั้ง"
                | "ต่อไป"
                | "จะ"
                | "เริ่ม"
                | "ช้าลง"
                | "นิดหนึ่ง"),
        )
    }

    pub fn settings_trigger_use_key(self) -> &'static str {
        self.pick("Use key", "ใช้ปุ่ม")
    }

    pub fn settings_gesture_hold(self) -> (&'static str, &'static str) {
        (
            self.pick("Hold", "กดค้าง"),
            self.pick(
                "Speak, let go, and the text lands at the cursor",
                th!("พูด" | "ปล่อย" | "แล้ว" | "วาง" | "ที่" | "เคอร์เซอร์"),
            ),
        )
    }

    pub fn settings_gesture_double_tap(self) -> (&'static str, &'static str) {
        (
            self.pick("Double-tap", "แตะสองครั้ง"),
            self.pick(
                "Speak for long without holding. Tap again to finish",
                th!("พูด"
                    | "ยาว"
                    | "ไม่ต้อง"
                    | "กดค้าง แตะ"
                    | "อีกครั้ง"
                    | "เพื่อ"
                    | "จบ"),
            ),
        )
    }

    pub fn settings_gesture_note(self) -> (&'static str, &'static str) {
        (
            self.pick("Shift + hold", "Shift + กดค้าง"),
            self.pick(
                "Keep it as a Note, nothing is pasted",
                th!("จด" | "เป็น" | "โน้ต" | "ไม่" | "วาง" | "ลง" | "แอป"),
            ),
        )
    }

    pub fn settings_trigger_esc(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::English => (
                "Press ",
                "Esc",
                " to cancel at any time. Nothing is pasted.",
            ),
            Self::Thai => (
                "กด ",
                "Esc",
                th!(" เพื่อ"
                    | "ยกเลิก"
                    | "ได้"
                    | "ทุกเมื่อ ข้อความ"
                    | "จะ"
                    | "ไม่ถูก"
                    | "วาง"),
            ),
        }
    }

    pub fn model_chip_ready(self, name: &str) -> String {
        match self {
            Self::English => format!("Ready · model {name}"),
            Self::Thai => format!("พร้อมใช้ · โมเดล {name}"),
        }
    }

    pub fn model_chip(self, chip: crate::model_chip::ModelChip) -> String {
        use crate::model_chip::{MODEL_NAME, ModelChip};
        match chip {
            ModelChip::Ready => self.model_chip_ready(MODEL_NAME),
            ModelChip::Loading => self.model_chip_loading().to_owned(),
            ModelChip::Downloading { percent } => self.model_chip_downloading(percent),
            ModelChip::Verifying => self.model_chip_verifying().to_owned(),
            ModelChip::Missing => self.model_chip_missing().to_owned(),
        }
    }

    pub fn model_done_badge(self) -> &'static str {
        self.pick("Ready", "พร้อมแล้ว")
    }

    pub fn model_chip_loading(self) -> &'static str {
        self.pick("Loading the model", "กำลังโหลดโมเดล")
    }

    pub fn model_chip_downloading(self, percent: u64) -> String {
        match self {
            Self::English => format!("Downloading the model · {percent}%"),
            Self::Thai => format!("กำลังดาวน์โหลดโมเดล · {percent}%"),
        }
    }

    pub fn model_chip_verifying(self) -> &'static str {
        self.pick("Checking the model", "กำลังตรวจสอบโมเดล")
    }

    pub fn model_chip_missing(self) -> &'static str {
        self.pick("Model not downloaded", "ยังไม่ได้ดาวน์โหลดโมเดล")
    }

    pub fn history_private(self) -> &'static str {
        self.pick("Kept on this Mac only", "เก็บไว้ในเครื่องนี้เท่านั้น")
    }

    pub fn history_footer_found(self, matched: usize, total: usize) -> String {
        match self {
            Self::English => format!("{matched} of {total} entries"),
            Self::Thai => format!("เจอ {matched} จาก {total} รายการ"),
        }
    }

    pub fn history_footer_total(self, total: usize) -> String {
        match self {
            Self::English => format!("{total} entries"),
            Self::Thai => format!("ทั้งหมด {total} รายการ"),
        }
    }

    pub fn history_footer_hint(self) -> &'static str {
        self.pick(
            "Paste the latest one from the bird icon in the menu bar",
            th!("วาง"
                | "อัน"
                | "ล่าสุด"
                | "ได้"
                | "จาก"
                | "ไอคอน"
                | "นก"
                | "บน"
                | "แถบเมนู"),
        )
    }

    pub fn history_day(self, day: crate::history_view::Day, current_year: i32) -> String {
        use crate::history_view::Day;
        use chrono::Datelike;
        match day {
            Day::Today => self.pick("Today", "วันนี้").to_owned(),
            Day::Yesterday => self.pick("Yesterday", "เมื่อวาน").to_owned(),
            Day::On(date) => match self {
                Self::English => {
                    if date.year() == current_year {
                        date.format("%b %-d").to_string()
                    } else {
                        date.format("%b %-d, %Y").to_string()
                    }
                }
                Self::Thai => {
                    let month = crate::history_view::thai_month_abbreviation(date.month());
                    if date.year() == current_year {
                        format!("{} {month}", date.day())
                    } else {
                        format!("{} {month} {}", date.day(), date.year() + 543)
                    }
                }
            },
        }
    }
}

fn group_thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

pub fn download_size(bytes: u64) -> String {
    let tenths = (bytes + 50_000_000) / 100_000_000;
    if tenths.is_multiple_of(10) {
        format!("{} GB", tenths / 10)
    } else {
        format!("{}.{} GB", tenths / 10, tenths % 10)
    }
}
