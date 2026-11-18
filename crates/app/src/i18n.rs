use platform::{Permission, PermissionReport, PermissionStatus};

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

    pub fn all_set(self) -> &'static str {
        self.pick(
            "All set. Hold Right Option and speak.",
            "พร้อมใช้งานแล้ว กด Right Option ค้างไว้แล้วพูดได้เลย",
        )
    }

    pub fn menu_setup(self) -> &'static str {
        self.pick("Set up permissions...", "ตั้งค่าสิทธิ์...")
    }

    pub fn menu_all_granted(self) -> &'static str {
        self.pick("All permissions granted", "ได้รับสิทธิ์ครบแล้ว")
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

    pub fn permission_why(self, permission: Permission) -> &'static str {
        match permission {
            Permission::Microphone => self.pick(
                "Hears you while you hold Right Option. Audio never leaves this Mac.",
                "ใช้ฟังเสียงของคุณขณะกด Right Option ค้างไว้ เสียงจะไม่ถูกส่งออกจากเครื่องนี้",
            ),
            Permission::Accessibility => self.pick(
                "Pastes the transcribed text into the app you are typing in.",
                "ใช้วางข้อความที่ถอดเสียงได้ลงในแอปที่คุณกำลังพิมพ์",
            ),
            Permission::InputMonitoring => self.pick(
                "Notices when you hold Right Option so PimSiang knows when to listen.",
                "ใช้ตรวจว่าคุณกด Right Option ค้างไว้ เพื่อให้ PimSiang รู้ว่าต้องเริ่มฟังเมื่อไร",
            ),
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
}
