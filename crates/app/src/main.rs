#[cfg(target_os = "macos")]
mod logging;
#[cfg(target_os = "macos")]
mod tray;

#[cfg(target_os = "macos")]
mod macos {
    use std::path::PathBuf;
    use std::sync::{Arc, mpsc};
    use std::time::Duration;

    use anyhow::{Context, Result};
    use app::controller::{Config, Controller, Input};
    use audio::MicCapture;
    use clap::Parser;
    use engine::{Engine, LanguageMode, ModelFiles, OnnxEngine, Precision};
    use platform::{Platform, Status};
    use platform_macos::MacPlatform;
    use tray_icon::menu::MenuEvent;
    use winit::application::ApplicationHandler;
    use winit::event::{StartCause, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
    use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
    use winit::window::WindowId;

    use crate::logging;
    use crate::tray::{MenuAction, MenuBar, open_folder};

    #[derive(Parser)]
    #[command(about = "PimSiang: hold Right Option and speak Thai")]
    struct Args {
        #[arg(long, env = "PIMSIANG_MODEL_DIR")]
        model_dir: PathBuf,
        #[arg(long, default_value = "int8")]
        precision: Precision,
        #[arg(long, default_value = "th-mix")]
        language: LanguageMode,
        #[arg(long, default_value_t = 250)]
        hold_ms: u64,
    }

    enum UserEvent {
        Status(Status),
        Menu(MenuEvent),
        Fatal(String),
    }

    struct App {
        log_dir: PathBuf,
        menu_bar: Option<MenuBar>,
        failure: Option<String>,
    }

    impl ApplicationHandler<UserEvent> for App {
        fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
            if cause == StartCause::Init {
                match MenuBar::new() {
                    Ok(menu_bar) => self.menu_bar = Some(menu_bar),
                    Err(error) => {
                        tracing::error!(%error, "cannot create the menu bar icon");
                        self.failure = Some(format!("{error:#}"));
                        event_loop.exit();
                    }
                }
            }
        }

        fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

        fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}

        fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
            match event {
                UserEvent::Status(status) => {
                    if let Some(menu_bar) = &self.menu_bar
                        && let Err(error) = menu_bar.show(status)
                    {
                        tracing::warn!(%error, "cannot update the menu bar icon");
                    }
                }
                UserEvent::Menu(event) => {
                    let action = self.menu_bar.as_ref().and_then(|bar| bar.action(&event.id));
                    match action {
                        Some(MenuAction::OpenLogFolder) => {
                            if let Err(error) = open_folder(&self.log_dir) {
                                tracing::warn!(%error, "cannot open the log folder");
                            }
                        }
                        Some(MenuAction::Quit) => event_loop.exit(),
                        None => {}
                    }
                }
                UserEvent::Fatal(message) => {
                    tracing::error!(%message, "stopping");
                    self.failure = Some(message);
                    event_loop.exit();
                }
            }
        }
    }

    pub fn run() -> Result<()> {
        let args = Args::parse();
        let log_dir = logging::log_dir()?;
        let _guard = logging::init(&log_dir)?;
        tracing::info!(model_dir = %args.model_dir.display(), "starting PimSiang");

        let event_loop = EventLoop::<UserEvent>::with_user_event()
            .with_activation_policy(ActivationPolicy::Accessory)
            .build()
            .context("creating the event loop")?;
        let proxy = event_loop.create_proxy();
        let menu_proxy = proxy.clone();
        MenuEvent::set_event_handler(Some(move |event| {
            let _ = menu_proxy.send_event(UserEvent::Menu(event));
        }));

        std::thread::Builder::new()
            .name("utterance-controller".to_owned())
            .spawn({
                let proxy = proxy.clone();
                move || {
                    if let Err(error) = run_controller(&args, &proxy) {
                        let _ = proxy.send_event(UserEvent::Fatal(format!("{error:#}")));
                    }
                }
            })
            .context("starting the controller thread")?;

        let mut app = App {
            log_dir,
            menu_bar: None,
            failure: None,
        };
        event_loop
            .run_app(&mut app)
            .context("running the event loop")?;
        match app.failure {
            Some(message) => Err(anyhow::anyhow!(message)),
            None => Ok(()),
        }
    }

    fn run_controller(args: &Args, proxy: &EventLoopProxy<UserEvent>) -> Result<()> {
        let (tx, rx) = mpsc::channel::<Input>();

        let status_proxy = proxy.clone();
        let mut platform = MacPlatform::new(Box::new(move |status| {
            let _ = status_proxy.send_event(UserEvent::Status(status));
        }));
        let mut engine = OnnxEngine::new(ModelFiles::new(&args.model_dir, args.precision));
        engine.load().context("loading the Model")?;
        tracing::info!("Model loaded");

        let trigger_tx = tx.clone();
        platform
            .listen_trigger(Box::new(move |event| {
                let _ = trigger_tx.send(Input::Trigger(event));
            }))
            .context("watching the Trigger key")?;

        let audio_tx = tx;
        let microphone = MicCapture::new(Arc::new(move |samples| {
            let _ = audio_tx.send(Input::Audio(samples));
        }));

        let controller = Controller::new(
            platform,
            engine,
            microphone,
            Config {
                hold_threshold: Duration::from_millis(args.hold_ms),
                language: args.language,
            },
        );
        controller.run(rx);
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn main() -> anyhow::Result<()> {
    macos::run()
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("PimSiang runs on macOS only");
}
