use objc2_app_kit::NSSound;
use objc2_foundation::NSString;
use platform::Sound;

pub fn play(sound: Sound) {
    let name = match sound {
        Sound::Start => "Tink",
        Sound::Stop => "Pop",
    };
    match NSSound::soundNamed(&NSString::from_str(name)) {
        Some(sound) => {
            if !sound.play() {
                tracing::warn!(name, "the system sound did not play");
            }
        }
        None => tracing::warn!(name, "the system sound was not found"),
    }
}
