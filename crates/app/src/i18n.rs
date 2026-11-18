use platform::{Permission, PermissionReport, PermissionStatus, TriggerKey};
use store::Kind;

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
            "PimSiang ต้องการสิทธิ์สามอย่างเพื่อพิมพ์สิ่งที่คุณพูด เมื่ออนุญาตครบแล้วหน้าต่างนี้จะปิดเอง",
        )
    }

    pub fn terminal_note(self) -> &'static str {
        self.pick(
            "PimSiang is running from a terminal, so these permissions belong to the terminal app that started it. Grant them to that app.",
            "PimSiang กำลังทำงานผ่านเทอร์มินัล สิทธิ์เหล่านี้จึงเป็นของแอปเทอร์มินัลที่เปิดมัน ให้อนุญาตกับแอปนั้น",
        )
    }

    pub fn all_set(self, key: TriggerKey) -> String {
        let key = key.label();
        match self {
            Self::English => format!("All set. Hold {key} and speak."),
            Self::Thai => format!("พร้อมใช้งานแล้ว กด {key} ค้างไว้แล้วพูดได้เลย"),
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
            self.pick("Missing permissions: ", "ขาดสิทธิ์: "),
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
                format!("ใช้ฟังเสียงของคุณขณะกด {key} ค้างไว้ เสียงจะไม่ถูกส่งออกจากเครื่องนี้")
            }
            (Permission::Accessibility, _) => self
                .pick(
                    "Pastes the transcribed text into the app you are typing in.",
                    "ใช้วางข้อความที่ถอดเสียงได้ลงในแอปที่คุณกำลังพิมพ์",
                )
                .to_owned(),
            (Permission::InputMonitoring, Self::English) => {
                format!("Notices when you hold {key} so PimSiang knows when to listen.")
            }
            (Permission::InputMonitoring, Self::Thai) => {
                format!("ใช้ตรวจว่าคุณกด {key} ค้างไว้ เพื่อให้ PimSiang รู้ว่าต้องเริ่มฟังเมื่อไร")
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
            "PimSiang ดาวน์โหลดโมเดลถอดเสียงครั้งเดียว ขนาดประมาณ 1 GB กรุณาเชื่อมต่ออินเทอร์เน็ตไว้ หากการดาวน์โหลดสะดุดจะต่อจากเดิมได้",
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
        self.pick("Checking the download...", "กำลังตรวจสอบไฟล์ที่ดาวน์โหลด...")
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

    pub fn settings_trigger_key(self) -> &'static str {
        self.pick("Trigger key", "ปุ่มสั่งพูด")
    }

    pub fn settings_trigger_key_help(self) -> &'static str {
        self.pick(
            "Hold it to talk, double-tap it for hands-free, hold it with Shift for a Quick note.",
            "กดค้างเพื่อพูด แตะสองครั้งเพื่อพูดต่อเนื่อง กดค้างพร้อม Shift เพื่อจดเป็นโน้ตด่วน",
        )
    }

    pub fn settings_language_mode(self) -> &'static str {
        self.pick("Language mode", "โหมดภาษา")
    }

    pub fn settings_language_mix(self) -> &'static str {
        self.pick(
            "th-mix: Thai, with English words written in English",
            "th-mix: ภาษาไทย คำอังกฤษเขียนเป็นตัวอักษรอังกฤษ",
        )
    }

    pub fn settings_language_thai(self) -> &'static str {
        self.pick(
            "th-TH: Thai only, English words written in Thai",
            "th-TH: ภาษาไทยล้วน คำอังกฤษเขียนทับศัพท์เป็นไทย",
        )
    }

    pub fn settings_language_help(self) -> &'static str {
        self.pick("Takes effect on your next Utterance.", "มีผลกับการพูดครั้งถัดไป")
    }

    pub fn settings_sounds(self) -> &'static str {
        self.pick(
            "Play a sound when listening starts and stops",
            "เล่นเสียงเมื่อเริ่มและหยุดฟัง",
        )
    }

    pub fn settings_clipboard_restore(self) -> &'static str {
        self.pick(
            "Restore the clipboard after pasting",
            "คืนค่าคลิปบอร์ดเดิมหลังวางข้อความ",
        )
    }

    pub fn settings_launch_at_login(self) -> &'static str {
        self.pick("Open PimSiang at login", "เปิด PimSiang เมื่อเข้าสู่ระบบ")
    }

    pub fn settings_login_unbundled(self) -> &'static str {
        self.pick(
            "Only works from the installed PimSiang app, not when it runs from a terminal.",
            "ใช้ได้เฉพาะแอป PimSiang ที่ติดตั้งแล้ว ไม่รองรับเมื่อรันจากเทอร์มินัล",
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
                "เปลี่ยนการเปิดตอนเข้าสู่ระบบไม่สำเร็จ: "
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
            "ยังไม่มีรายการ ทุกอย่างที่คุณพูดจะถูกเก็บไว้ที่นี่ ในเครื่องนี้เท่านั้น",
        )
    }

    pub fn history_no_match(self) -> &'static str {
        self.pick("No entries match your search.", "ไม่พบรายการที่ตรงกับคำค้นหา")
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
            "ลบประวัติทั้งหมด? การลบนี้ย้อนกลับไม่ได้",
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
            "ใช้งานประวัติไม่ได้: เปิดฐานข้อมูลไม่สำเร็จ",
        )
    }
}
