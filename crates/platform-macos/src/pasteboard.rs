use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardTypeString, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSData, NSString};
use platform::clipboard::Pasteboard;

pub type Item = Vec<(String, Vec<u8>)>;

pub struct GeneralPasteboard {
    pasteboard: Retained<NSPasteboard>,
}

impl GeneralPasteboard {
    pub fn new() -> Self {
        Self {
            pasteboard: NSPasteboard::generalPasteboard(),
        }
    }
}

impl Pasteboard for GeneralPasteboard {
    type Snapshot = Vec<Item>;

    fn change_count(&self) -> i64 {
        self.pasteboard.changeCount() as i64
    }

    fn snapshot(&mut self) -> Option<Self::Snapshot> {
        let Some(items) = self.pasteboard.pasteboardItems() else {
            return Some(Vec::new());
        };
        Some(
            items
                .iter()
                .map(|item| {
                    item.types()
                        .iter()
                        .filter_map(|kind| {
                            item.dataForType(&kind)
                                .map(|data| (kind.to_string(), data.to_vec()))
                        })
                        .collect()
                })
                .collect(),
        )
    }

    fn write_text(&mut self, text: &str) -> Result<(), String> {
        self.pasteboard.clearContents();
        self.pasteboard
            .setString_forType(&NSString::from_str(text), string_type())
            .then_some(())
            .ok_or_else(|| "the clipboard refused the text".to_owned())
    }

    fn restore(&mut self, snapshot: Self::Snapshot) -> Result<(), String> {
        self.pasteboard.clearContents();
        let items: Vec<Retained<NSPasteboardItem>> = snapshot
            .iter()
            .map(|entries| {
                let item = NSPasteboardItem::new();
                for (kind, bytes) in entries {
                    item.setData_forType(&NSData::with_bytes(bytes), &NSString::from_str(kind));
                }
                item
            })
            .collect();
        if items.is_empty() {
            return Ok(());
        }
        let writers: Vec<Retained<ProtocolObject<dyn NSPasteboardWriting>>> = items
            .into_iter()
            .map(ProtocolObject::from_retained)
            .collect();
        self.pasteboard
            .writeObjects(&NSArray::from_retained_slice(&writers))
            .then_some(())
            .ok_or_else(|| "the clipboard refused the previous contents".to_owned())
    }
}

#[allow(unsafe_code)]
fn string_type() -> &'static NSString {
    unsafe { NSPasteboardTypeString }
}
