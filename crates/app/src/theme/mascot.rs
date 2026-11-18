use egui::{ColorImage, Id, TextureHandle, TextureOptions, Theme, Ui, Vec2};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Pose {
    Default,
    Writing,
    Confused,
    Sleeping,
    Waving,
    Oops,
    Searching,
    HeadDefault,
}

macro_rules! art {
    ($name:literal) => {
        (
            include_bytes!(concat!("../../assets/mascot/", $name, ".png")).as_slice(),
            include_bytes!(concat!("../../assets/mascot/", $name, "-dark.png")).as_slice(),
        )
    };
}

impl Pose {
    fn bytes(self, theme: Theme) -> &'static [u8] {
        let (light, dark) = match self {
            Pose::Default => art!("default"),
            Pose::Writing => art!("writing"),
            Pose::Confused => art!("confused"),
            Pose::Sleeping => art!("sleeping"),
            Pose::Waving => art!("waving"),
            Pose::Oops => art!("oops"),
            Pose::Searching => art!("searching"),
            Pose::HeadDefault => art!("head-default"),
        };
        match theme {
            Theme::Light => light,
            Theme::Dark => dark,
        }
    }
}

fn decode(bytes: &[u8]) -> Option<ColorImage> {
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .ok()?
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    Some(ColorImage::from_rgba_unmultiplied(size, image.as_raw()))
}

fn texture(ui: &Ui, pose: Pose) -> Option<TextureHandle> {
    let theme = ui.ctx().theme();
    let id = Id::unique(("pimsiang-mascot", pose, theme == Theme::Dark));
    if let Some(handle) = ui.ctx().data(|data| data.get_temp::<TextureHandle>(id)) {
        return Some(handle);
    }
    let image = decode(pose.bytes(theme))?;
    let handle = ui.ctx().load_texture(
        format!("mascot-{pose:?}-{theme:?}"),
        image,
        TextureOptions::LINEAR,
    );
    ui.ctx()
        .data_mut(|data| data.insert_temp(id, handle.clone()));
    Some(handle)
}

pub fn show(ui: &mut Ui, pose: Pose, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    if let Some(handle) = texture(ui, pose) {
        egui::Image::new(&handle)
            .fit_to_exact_size(Vec2::splat(size))
            .paint_at(ui, rect);
    }
}
