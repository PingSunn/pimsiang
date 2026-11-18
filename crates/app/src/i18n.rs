use platform::{
    Permission, PermissionReport, PermissionStatus, PillError, PillNotice, PillState, Status,
    TriggerKey,
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
        self.pick("Welcome to PimSiang", "ยินดีต้อนรับสู่ PimSiang")
    }

    pub fn intro(self) -> &'static str {
        self.pick(
            "PimSiang needs three permissions to type what you say. This window closes by itself once they are granted.",
            th!("PimSiang ต้องการ" | "สิทธิ์" | "สามอย่าง" | "เพื่อ" | "พิมพ์" | "สิ่งที่" | "คุณ" | "พูด เมื่อ" | "อนุญาต" | "ครบแล้ว" | "หน้าต่าง" | "นี้" | "จะ" | "ปิดเอง"),
        )
    }

    pub fn terminal_note(self) -> &'static str {
        self.pick(
            "PimSiang is running from a terminal, so these permissions belong to the terminal app that started it. Grant them to that app.",
            th!("PimSiang กำลัง" | "ทำงาน" | "ผ่าน" | "เทอร์มินัล สิทธิ์" | "เหล่านี้" | "จึง" | "เป็นของ" | "แอป" | "เทอร์มินัล" | "ที่" | "เปิด" | "มัน ให้" | "อนุญาต" | "กับ" | "แอป" | "นั้น"),
        )
    }

    pub fn all_set(self, key: TriggerKey) -> String {
        let key = key.label();
        match self {
            Self::English => format!("All set. Hold {key} and speak."),
            Self::Thai => format!(
                th!("พร้อม"
                    | "ใช้งาน"
                    | "แล้ว กด {key} ค้างไว้"
                    | "แล้ว"
                    | "พูด"
                    | "ได้เลย"),
                key = key
            ),
        }
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

    pub fn model_explain(self) -> &'static str {
        self.pick(
            "PimSiang downloads its speech model once, about 1 GB. Keep this Mac online. If the download is interrupted it resumes where it stopped.",
            th!("PimSiang ดาวน์โหลด" | "โมเดล" | "ถอดเสียง" | "ครั้งเดียว ขนาด" | "ประมาณ 1 GB กรุณา" | "เชื่อมต่อ" | "อินเทอร์เน็ต" | "ไว้ หาก" | "การ" | "ดาวน์โหลด" | "สะดุด" | "จะ" | "ต่อ" | "จาก" | "เดิม" | "ได้"),
        )
    }

    pub fn model_downloading(self, done: u64, total: u64) -> String {
        let percent = (done * 100).checked_div(total).unwrap_or(0);
        format!(
            "{}{} / {} MB ({percent}%)",
            self.pick("Downloading... ", "กำลังดาวน์โหลด... "),
            done / 1_000_000,
            total / 1_000_000,
        )
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
        self.pick("Quit PimSiang", "ออกจาก PimSiang")
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
        self.pick("Transcribing…", "กำลังถอดเสียง…")
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
            PillNotice::QuickNoteSaved => self.pick("Saved as a Note", "บันทึกเป็นโน้ตแล้ว"),
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
            PillState::Partial(text) => Some(text.clone()),
            PillState::Transcribing => Some(self.pill_transcribing().to_owned()),
            PillState::Notice(notice) => Some(self.pill_notice(*notice).to_owned()),
            PillState::Error(error) => Some(self.pill_error(*error)),
        }
    }

    pub fn menu_history(self) -> &'static str {
        self.pick("History...", "ประวัติ...")
    }

    pub fn menu_paste_last(self) -> &'static str {
        self.pick("Paste last transcript", "วางข้อความล่าสุดอีกครั้ง")
    }

    pub fn menu_settings(self) -> &'static str {
        self.pick("Settings...", "ตั้งค่า...")
    }

    pub fn settings_title(self) -> &'static str {
        self.pick("PimSiang settings", "การตั้งค่า PimSiang")
    }

    pub fn settings_applies_now(self) -> &'static str {
        self.pick("Changes apply straight away.", "การเปลี่ยนแปลงมีผลทันที")
    }

    pub fn section_general(self) -> &'static str {
        self.pick("General", "ทั่วไป")
    }

    pub fn section_trigger(self) -> &'static str {
        self.pick("Trigger", "ปุ่มสั่งพูด")
    }

    pub fn section_sounds_clipboard(self) -> &'static str {
        self.pick("Sounds & Clipboard", "เสียงและคลิปบอร์ด")
    }

    pub fn history_heading(self) -> &'static str {
        self.pick("History", "ประวัติ")
    }

    pub fn setup_progress(self, done: usize, total: usize) -> String {
        match self {
            Self::English => format!("{done} of {total} steps ready"),
            Self::Thai => format!("พร้อมแล้ว {done} จาก {total} ขั้นตอน"),
        }
    }

    pub fn settings_trigger_key(self) -> &'static str {
        self.pick("Trigger key", "ปุ่มสั่งพูด")
    }

    pub fn settings_trigger_key_help(self) -> &'static str {
        self.pick(
            "Hold it to talk, double-tap it for hands-free, hold it with Shift for a Quick note.",
            th!("กด"
                | "ค้าง"
                | "เพื่อ"
                | "พูด แตะ"
                | "สองครั้ง"
                | "เพื่อ"
                | "พูด"
                | "ต่อเนื่อง กด"
                | "ค้าง"
                | "พร้อม Shift เพื่อ"
                | "จด"
                | "เป็น"
                | "โน้ต"
                | "ด่วน"),
        )
    }

    pub fn settings_language_mode(self) -> &'static str {
        self.pick("Language mode", "โหมดภาษา")
    }

    pub fn settings_language_mix(self) -> &'static str {
        self.pick(
            "th-mix: Thai, with English words written in English",
            th!("th-mix: ภาษา"
                | "ไทย คำ"
                | "อังกฤษ"
                | "เขียน"
                | "เป็น"
                | "ตัวอักษร"
                | "อังกฤษ"),
        )
    }

    pub fn settings_language_thai(self) -> &'static str {
        self.pick(
            "th-TH: Thai only, English words written in Thai",
            th!("th-TH: ภาษา"
                | "ไทย"
                | "ล้วน คำ"
                | "อังกฤษ"
                | "เขียน"
                | "ทับศัพท์"
                | "เป็น"
                | "ไทย"),
        )
    }

    pub fn settings_language_help(self) -> &'static str {
        self.pick(
            "Takes effect on your next Utterance.",
            th!("มีผล" | "กับ" | "การ" | "พูด" | "ครั้ง" | "ถัดไป"),
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

    pub fn settings_unload_when_idle(self) -> &'static str {
        self.pick(
            "Free the Model's memory after 10 idle minutes",
            th!("คืน"
                | "หน่วยความจำ"
                | "ของ"
                | "โมเดล"
                | "เมื่อ"
                | "ไม่ได้"
                | "ใช้งาน 10 นาที"),
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
        self.pick("Clear all...", "ลบทั้งหมด...")
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
}
