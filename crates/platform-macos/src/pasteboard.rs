use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::{NSPasteboard, NSPasteboardItem, NSPasteboardTypeString, NSPasteboardWriting};
use objc2_foundation::{NSArray, NSData, NSString};
use platform::clipboard::{Gathered, Item, Limits, Pasteboard, gather};

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
        let items: Vec<_> = items.iter().collect();
        let types: Vec<Vec<String>> = items
            .iter()
            .map(|item| item.types().iter().map(|kind| kind.to_string()).collect())
            .collect();
        let gathered = gather(&types, &Limits::default(), |index, kind| {
            items[index]
                .dataForType(&NSString::from_str(kind))
                .map(|data| data.to_vec())
        });
        match gathered {
            Gathered::Items(items) => Some(items),
            Gathered::Sensitive => {
                tracing::info!("the clipboard holds a concealed item; not restoring it");
                Some(Vec::new())
            }
            Gathered::OverBudget => {
                tracing::info!("the clipboard is too large to save; not restoring it");
                None
            }
        }
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
