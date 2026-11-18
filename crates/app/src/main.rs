#[cfg(target_os = "macos")]
mod logging;
#[cfg(target_os = "macos")]
mod onboarding_window;
#[cfg(target_os = "macos")]
mod tray;

#[cfg(target_os = "macos")]
mod macos {
    use std::path::PathBuf;
    use std::sync::mpsc::Sender;
    use std::sync::{Arc, mpsc};
    use std::time::{Duration, Instant};

    use anyhow::{Context, Result};
    use app::controller::{Config, Controller, Input, PermissionCommand};
    use app::i18n::Language;
    use app::onboarding;
    use audio::MicCapture;
    use clap::Parser;
    use engine::{LanguageMode, ModelFiles, OnnxEngine, Precision};
    use objc2::MainThreadMarker;
    use platform::{PermissionReport, PillState, Sound, Status};
    use platform_macos::{MacPlatform, PillPanel, play_sound};
    use tray_icon::menu::MenuEvent;
    use winit::application::ApplicationHandler;
    use winit::event::{StartCause, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
    use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
    use winit::window::WindowId;

    use crate::logging;
    use crate::onboarding_window::{OnboardingWindow, Outcome};
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

    const PERMISSION_POLL: Duration = Duration::from_secs(1);
    const SETUP_CLOSE_DELAY: Duration = Duration::from_millis(1_500);

    enum UserEvent {
        Status(Status),
        Permissions(PermissionReport),
        Pill(PillState),
        Sound(Sound),
        Menu(MenuEvent),
        Fatal(String),
    }

    struct App {
        log_dir: PathBuf,
        menu_bar: Option<MenuBar>,
        pill: Option<PillPanel>,
        failure: Option<String>,
        language: Language,
        bundled: bool,
        inputs: Sender<Input>,
        report: Option<PermissionReport>,
        setup: Option<OnboardingWindow>,
        close_setup_at: Option<Instant>,
    }

    impl App {
        fn open_setup(&mut self, event_loop: &ActiveEventLoop) {
            if let Some(setup) = &self.setup {
                setup.focus();
                return;
            }
            match OnboardingWindow::open(event_loop, self.language.window_title()) {
                Ok(setup) => self.setup = Some(setup),
                Err(error) => tracing::error!(%error, "cannot open the setup window"),
            }
        }

        fn setup_view(&self) -> onboarding::View {
            let report = self.report.unwrap_or(PermissionReport {
                microphone: platform::PermissionStatus::NotDetermined,
                accessibility: platform::PermissionStatus::NotDetermined,
                input_monitoring: platform::PermissionStatus::NotDetermined,
            });
            onboarding::view(self.language, &report, self.bundled)
        }

        fn on_permissions(&mut self, event_loop: &ActiveEventLoop, report: PermissionReport) {
            let first = self.report.is_none();
            self.report = Some(report);
            if let Some(menu_bar) = &self.menu_bar
                && let Err(error) = menu_bar.show_permissions(&report)
            {
                tracing::warn!(%error, "cannot update the menu bar permissions");
            }
            if first && !report.all_granted() {
                self.open_setup(event_loop);
            }
            if let Some(setup) = &self.setup {
                setup.request_redraw();
                self.close_setup_at = report
                    .all_granted()
                    .then(|| Instant::now() + SETUP_CLOSE_DELAY);
            }
        }
    }

    impl ApplicationHandler<UserEvent> for App {
        fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
            if cause == StartCause::Init {
                match MenuBar::new(self.language) {
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

        fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
            match self.close_setup_at {
                Some(at) if Instant::now() >= at => {
                    self.close_setup_at = None;
                    self.setup = None;
                    event_loop.set_control_flow(ControlFlow::Wait);
                }
                Some(at) => event_loop.set_control_flow(ControlFlow::WaitUntil(at)),
                None => event_loop.set_control_flow(ControlFlow::Wait),
            }
        }

        fn window_event(&mut self, _: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
            let view = self.setup_view();
            let Some(setup) = self.setup.as_mut().filter(|setup| setup.id() == id) else {
                return;
            };
            match setup.handle(&event, &view) {
                Outcome::Idle => {}
                Outcome::Close => {
                    self.setup = None;
                    self.close_setup_at = None;
                }
                Outcome::Grant(permission) => {
                    let _ = self
                        .inputs
                        .send(Input::Permission(PermissionCommand::Grant(permission)));
                }
            }
        }

        fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
            match event {
                UserEvent::Status(status) => {
                    if let Some(menu_bar) = &self.menu_bar
                        && let Err(error) = menu_bar.show(status)
                    {
                        tracing::warn!(%error, "cannot update the menu bar icon");
                    }
                }
                UserEvent::Permissions(report) => self.on_permissions(event_loop, report),
                UserEvent::Pill(state) => {
                    if self.pill.is_none() {
                        self.pill = MainThreadMarker::new().map(PillPanel::new);
                    }
                    if let Some(pill) = &self.pill {
                        pill.show(&state);
                    }
                }
                UserEvent::Sound(sound) => play_sound(sound),
                UserEvent::Menu(event) => {
                    let action = self.menu_bar.as_ref().and_then(|bar| bar.action(&event.id));
                    match action {
                        Some(MenuAction::SetupPermissions) => self.open_setup(event_loop),
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

        let (inputs, receiver) = mpsc::channel::<Input>();
        std::thread::Builder::new()
            .name("utterance-controller".to_owned())
            .spawn({
                let proxy = proxy.clone();
                let inputs = inputs.clone();
                move || {
                    if let Err(error) = run_controller(&args, &proxy, inputs, receiver) {
                        let _ = proxy.send_event(UserEvent::Fatal(format!("{error:#}")));
                    }
                }
            })
            .context("starting the controller thread")?;

        let mut app = App {
            log_dir,
            menu_bar: None,
            pill: None,
            failure: None,
            language: Language::from_tags(&platform_macos::system_languages()),
            bundled: std::env::current_exe()
                .map(|path| onboarding::is_bundled(&path))
                .unwrap_or(false),
            inputs,
            report: None,
            setup: None,
            close_setup_at: None,
        };
        event_loop
            .run_app(&mut app)
            .context("running the event loop")?;
        match app.failure {
            Some(message) => Err(anyhow::anyhow!(message)),
            None => Ok(()),
        }
    }

    fn run_controller(
        args: &Args,
        proxy: &EventLoopProxy<UserEvent>,
        tx: Sender<Input>,
        rx: mpsc::Receiver<Input>,
    ) -> Result<()> {
        let status_proxy = proxy.clone();
        let pill_proxy = proxy.clone();
        let permissions_proxy = proxy.clone();
        let sound_proxy = proxy.clone();
        let platform = MacPlatform::new(
            Box::new(move |status| {
                let _ = status_proxy.send_event(UserEvent::Status(status));
            }),
            Box::new(move |state| {
                let _ = pill_proxy.send_event(UserEvent::Pill(state));
            }),
            Box::new(move |report| {
                let _ = permissions_proxy.send_event(UserEvent::Permissions(report));
            }),
            Box::new(move |sound| {
                let _ = sound_proxy.send_event(UserEvent::Sound(sound));
            }),
        );
        let engine = OnnxEngine::new(ModelFiles::new(&args.model_dir, args.precision));

        let trigger_tx = tx.clone();
        let audio_tx = tx;
        let microphone = MicCapture::new(Arc::new(move |samples| {
            let _ = audio_tx.send(Input::Audio(samples));
        }));

        let mut controller = Controller::new(
            platform,
            engine,
            microphone,
            Config {
                hold_threshold: Duration::from_millis(args.hold_ms),
                language: args.language,
                ..Config::default()
            },
        )
        .watch_permissions(PERMISSION_POLL, move || {
            let trigger_tx = trigger_tx.clone();
            Box::new(move |event| {
                let _ = trigger_tx.send(Input::Trigger(event));
            })
        });
        controller.handle(Input::Tick(Instant::now()));
        controller.preload_model();
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
