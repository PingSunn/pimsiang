#[cfg(target_os = "macos")]
mod egui_surface;
#[cfg(target_os = "macos")]
mod history_window;
#[cfg(target_os = "macos")]
mod logging;
#[cfg(target_os = "macos")]
mod onboarding_window;
#[cfg(target_os = "macos")]
mod settings_window;
#[cfg(target_os = "macos")]
mod theme;
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
    use app::history_view;
    use app::i18n::Language;
    use app::model_download::{self, Manifest, ModelStatus};
    use app::onboarding;
    use app::preferences::Preferences;
    use app::single_instance::InstanceLock;
    use app::update;
    use audio::{MicCapture, MicrophoneChoice};
    use clap::Parser;
    use engine::{LanguageMode, ModelFiles, OnnxEngine, Precision};
    use objc2::MainThreadMarker;
    use platform::{MicrophoneMenu, PermissionReport, PillState, Sound, Status};
    use platform_macos::login_item::{self, LoginItem};
    use platform_macos::{MacPlatform, PillPanel, copy_text, play_sound};
    use store::{Entry, HistoryService, Settings};
    use tray_icon::menu::MenuEvent;
    use winit::application::ApplicationHandler;
    use winit::event::{StartCause, WindowEvent};
    use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
    use winit::platform::macos::{
        ActivationPolicy, ActiveEventLoopExtMacOS, EventLoopBuilderExtMacOS,
    };
    use winit::window::WindowId;

    use crate::history_window::{HistoryWindow, Outcome as HistoryOutcome};
    use crate::logging;
    use crate::onboarding_window::{OnboardingWindow, Outcome};
    use crate::settings_window::{Outcome as SettingsOutcome, SettingsWindow};
    use crate::tray::{MenuAction, MenuBar, open_folder};

    #[derive(Parser)]
    #[command(about = "PimSiang: hold the trigger key and speak Thai")]
    struct Args {
        #[arg(long, env = "PIMSIANG_MODEL_DIR")]
        model_dir: Option<PathBuf>,
        #[arg(long, default_value = "int8")]
        precision: Precision,
        #[arg(long)]
        language: Option<LanguageMode>,
        #[arg(long, default_value_t = 250)]
        hold_ms: u64,
    }

    const PERMISSION_POLL: Duration = Duration::from_secs(1);
    const SETUP_CLOSE_DELAY: Duration = Duration::from_millis(1_500);
    const QUIT_GRACE: Duration = Duration::from_secs(3);
    const HANDOVER: Duration = Duration::from_millis(250);

    enum UserEvent {
        Status(Status),
        Permissions(PermissionReport),
        Pill(PillState),
        Sound(Sound),
        Microphones(MicrophoneMenu),
        Update(Option<String>),
        Model(ModelStatus),
        HistoryEntries(Vec<Entry>),
        HistoryChanged,
        Menu(MenuEvent),
        Fatal(String),
    }

    struct App {
        log_dir: PathBuf,
        settings_path: Option<PathBuf>,
        menu_bar: Option<MenuBar>,
        pill: Option<PillPanel>,
        failure: Option<String>,
        language: Language,
        bundled: bool,
        model: ModelStatus,
        model_dir: PathBuf,
        custom_model: Option<ModelFiles>,
        model_prompted: bool,
        proxy: EventLoopProxy<UserEvent>,
        inputs: Sender<Input>,
        report: Option<PermissionReport>,
        setup: Option<OnboardingWindow>,
        close_setup_at: Option<Instant>,
        history: Option<Arc<HistoryService>>,
        history_window: Option<HistoryWindow>,
        preferences: Preferences,
        language_overridden: bool,
        settings_window: Option<SettingsWindow>,
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

        fn open_history(&mut self, event_loop: &ActiveEventLoop) {
            if let Some(window) = &self.history_window {
                window.focus();
                return;
            }
            match HistoryWindow::open(event_loop, self.language, self.history.is_none()) {
                Ok(window) => {
                    self.history_window = Some(window);
                    self.refresh_history();
                }
                Err(error) => tracing::error!(%error, "cannot open the History window"),
            }
        }

        fn open_settings(&mut self, event_loop: &ActiveEventLoop) {
            if let Some(window) = &self.settings_window {
                window.focus();
                return;
            }
            match SettingsWindow::open(
                event_loop,
                self.language,
                self.preferences,
                self.bundled,
                login_item::status(),
            ) {
                Ok(window) => self.settings_window = Some(window),
                Err(error) => tracing::error!(%error, "cannot open the settings window"),
            }
        }

        fn on_settings_window(&mut self, event: &WindowEvent) {
            let Some(window) = self.settings_window.as_mut() else {
                return;
            };
            match window.handle(event) {
                SettingsOutcome::Idle => {}
                SettingsOutcome::Close => self.settings_window = None,
                SettingsOutcome::OpenLoginItems => login_item::open_login_items_settings(),
                SettingsOutcome::Changed(changed) => self.change_preferences(changed),
            }
        }

        fn change_preferences(&mut self, mut changed: Preferences) {
            let mut error = None;
            if changed.launch_at_login != self.preferences.launch_at_login {
                match login_item::set(changed.launch_at_login) {
                    Ok(()) => {}
                    Err(reason) => {
                        tracing::warn!(%reason, "cannot change the login item");
                        changed.launch_at_login = self.preferences.launch_at_login;
                        error = Some(self.language.settings_login_failed(&reason));
                    }
                }
            }
            let keep_language =
                self.language_overridden && changed.language == self.preferences.language;
            if !keep_language {
                self.language_overridden = false;
            }
            self.preferences = changed;
            if let Some(path) = &self.settings_path
                && let Err(error) = Settings::update(path, |settings| {
                    changed.apply_keeping_language(settings, keep_language)
                })
            {
                tracing::warn!(%error, "cannot save the settings");
            }
            let _ = self.inputs.send(Input::Preferences(changed));
            if let Some(window) = self.settings_window.as_mut() {
                window.show_result(changed, login_item::status(), error);
            }
        }

        fn refresh_history(&self) {
            let (Some(service), Some(window)) = (&self.history, &self.history_window) else {
                return;
            };
            let query = window.query().to_owned();
            let limit = window.limit();
            let proxy = self.proxy.clone();
            service.run(move |history| match history.search(&query, 0, limit) {
                Ok(entries) => {
                    let _ = proxy.send_event(UserEvent::HistoryEntries(entries));
                }
                Err(error) => tracing::warn!(%error, "cannot read History"),
            });
        }

        fn paste_last(&self, event_loop: &ActiveEventLoop) {
            let Some(service) = &self.history else {
                return;
            };
            let handover = if self.history_window.is_some() {
                event_loop.hide_application();
                Some(HANDOVER)
            } else {
                None
            };
            let inputs = self.inputs.clone();
            service.run(move |history| match history.latest() {
                Ok(Some(entry)) => {
                    if let Some(wait) = handover {
                        std::thread::sleep(wait);
                    }
                    let _ = inputs.send(Input::PasteText(entry.text));
                }
                Ok(None) => {
                    let _ = inputs.send(Input::NoTranscript);
                }
                Err(error) => tracing::warn!(%error, "cannot read the last transcript"),
            });
        }

        fn on_history_window(&mut self, event: &WindowEvent) {
            let Some(window) = self.history_window.as_mut() else {
                return;
            };
            match window.handle(event) {
                HistoryOutcome::Idle => {}
                HistoryOutcome::Close => self.history_window = None,
                HistoryOutcome::Refresh => self.refresh_history(),
                HistoryOutcome::Copy(text) => {
                    if let Err(error) = copy_text(&text) {
                        tracing::warn!(%error, "cannot copy the History entry");
                    }
                }
                HistoryOutcome::Delete(id) => {
                    if let Some(service) = &self.history {
                        service.run(move |history| {
                            if let Err(error) = history.delete(id) {
                                tracing::warn!(%error, "cannot delete the History entry");
                            }
                        });
                    }
                    self.refresh_history();
                }
                HistoryOutcome::Clear => {
                    if let Some(service) = &self.history {
                        service.run(|history| {
                            if let Err(error) = history.clear() {
                                tracing::warn!(%error, "cannot clear History");
                            }
                        });
                    }
                    self.refresh_history();
                }
            }
        }

        fn choose_microphone(&mut self, choice: MicrophoneChoice) {
            if let Some(path) = &self.settings_path {
                let saved = Settings::update(path, |settings| {
                    settings.microphone = match &choice {
                        MicrophoneChoice::SystemDefault => None,
                        MicrophoneChoice::Device(id) => Some(id.clone()),
                    };
                });
                if let Err(error) = saved {
                    tracing::warn!(%error, "cannot save the microphone choice");
                }
            }
            let _ = self.inputs.send(Input::Microphone(choice));
        }

        fn setup_view(&self) -> onboarding::View {
            let report = self.report.unwrap_or(PermissionReport {
                microphone: platform::PermissionStatus::NotDetermined,
                accessibility: platform::PermissionStatus::NotDetermined,
                input_monitoring: platform::PermissionStatus::NotDetermined,
            });
            onboarding::view(
                self.language,
                self.preferences.trigger_key,
                &report,
                self.bundled,
                &self.model,
            )
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
            self.refresh_setup();
        }

        fn refresh_setup(&mut self) {
            if let Some(setup) = &self.setup {
                setup.request_redraw();
                let done = self.report.is_some_and(|report| report.all_granted())
                    && self.model == ModelStatus::Ready;
                self.close_setup_at = done.then(|| Instant::now() + SETUP_CLOSE_DELAY);
            }
        }

        fn on_model(&mut self, event_loop: &ActiveEventLoop, status: ModelStatus) {
            let failed = matches!(status, ModelStatus::Failed(_));
            let ready = status == ModelStatus::Ready;
            self.model = status;
            if !ready && (failed || !self.model_prompted) {
                self.model_prompted = true;
                self.open_setup(event_loop);
            }
            self.refresh_setup();
        }

        fn check_custom_model(&mut self, files: &ModelFiles) {
            let missing = model_download::missing_files(files);
            let status = if missing.is_empty() {
                let _ = self.inputs.send(Input::ModelReady);
                ModelStatus::Ready
            } else {
                ModelStatus::Failed(format!(
                    "{} is missing {}",
                    files.dir.display(),
                    missing.join(", ")
                ))
            };
            self.model = status.clone();
            let _ = self.proxy.send_event(UserEvent::Model(status));
        }

        fn start_model_download(&mut self) {
            self.model = ModelStatus::Downloading {
                done: 0,
                total: Manifest::pinned().total_bytes(),
            };
            let notify_proxy = self.proxy.clone();
            let inputs = self.inputs.clone();
            if let Err(error) =
                model_download::spawn(self.model_dir.clone(), Manifest::pinned(), move |status| {
                    if status == ModelStatus::Ready {
                        let _ = inputs.send(Input::ModelReady);
                    }
                    let _ = notify_proxy.send_event(UserEvent::Model(status));
                })
            {
                tracing::error!(%error, "cannot start the Model download");
                let status = ModelStatus::Failed(error.to_string());
                self.model = status.clone();
                let _ = self.proxy.send_event(UserEvent::Model(status));
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
            if self
                .history_window
                .as_ref()
                .is_some_and(|window| window.id() == id)
            {
                self.on_history_window(&event);
                return;
            }
            if self
                .settings_window
                .as_ref()
                .is_some_and(|window| window.id() == id)
            {
                self.on_settings_window(&event);
                return;
            }
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
                Outcome::RetryModel => match self.custom_model.clone() {
                    Some(files) => self.check_custom_model(&files),
                    None => self.start_model_download(),
                },
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
                        pill.show(self.language.pill_text(&state).as_deref());
                    }
                }
                UserEvent::Sound(sound) => play_sound(sound),
                UserEvent::Microphones(menu) => {
                    if let Some(menu_bar) = &self.menu_bar
                        && let Err(error) = menu_bar.show_microphones(&menu)
                    {
                        tracing::warn!(%error, "cannot update the microphone menu");
                    }
                }
                UserEvent::Model(status) => self.on_model(event_loop, status),
                UserEvent::HistoryChanged => self.refresh_history(),
                UserEvent::HistoryEntries(entries) => {
                    if let Some(window) = self.history_window.as_mut() {
                        window.set_entries(entries);
                    }
                }
                UserEvent::Update(version) => {
                    if let Some(menu_bar) = &self.menu_bar
                        && let Err(error) = menu_bar.show_update(version.as_deref())
                    {
                        tracing::warn!(%error, "cannot update the update notice");
                    }
                }
                UserEvent::Menu(event) => {
                    let action = self.menu_bar.as_ref().and_then(|bar| bar.action(&event.id));
                    match action {
                        Some(MenuAction::Microphone(choice)) => self.choose_microphone(choice),
                        Some(MenuAction::SetupPermissions) => self.open_setup(event_loop),
                        Some(MenuAction::OpenHistory) => self.open_history(event_loop),
                        Some(MenuAction::PasteLast) => self.paste_last(event_loop),
                        Some(MenuAction::OpenSettings) => self.open_settings(event_loop),
                        Some(MenuAction::CopyUpgradeCommand) => {
                            match copy_text(update::UPGRADE_COMMAND) {
                                Ok(()) => {
                                    if let Some(menu_bar) = &self.menu_bar {
                                        menu_bar.flash_update_copied();
                                    }
                                }
                                Err(error) => {
                                    tracing::warn!(%error, "cannot copy the upgrade command");
                                }
                            }
                        }
                        Some(MenuAction::DismissUpdate) => {
                            if let Some(path) = &self.settings_path
                                && let Err(error) = update::dismiss(path)
                            {
                                tracing::warn!(%error, "cannot save the dismissed update");
                            }
                            if let Some(menu_bar) = &self.menu_bar
                                && let Err(error) = menu_bar.show_update(None)
                            {
                                tracing::warn!(%error, "cannot hide the update notice");
                            }
                        }
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
        let _instance = match lock_path().map(|path| InstanceLock::acquire(&path)) {
            Some(Ok(Some(lock))) => Some(lock),
            Some(Ok(None)) => {
                tracing::info!("PimSiang is already running, exiting");
                return Ok(());
            }
            Some(Err(error)) => {
                tracing::warn!(%error, "cannot take the single-instance lock");
                None
            }
            None => None,
        };
        let model = ModelSource::choose(&args);
        tracing::info!(model_dir = %model.dir.display(), download = model.download, "starting PimSiang");

        let event_loop = EventLoop::<UserEvent>::with_user_event()
            .with_activation_policy(ActivationPolicy::Accessory)
            .build()
            .context("creating the event loop")?;
        let proxy = event_loop.create_proxy();
        let menu_proxy = proxy.clone();
        MenuEvent::set_event_handler(Some(move |event| {
            let _ = menu_proxy.send_event(UserEvent::Menu(event));
        }));

        let history = history_path().and_then(|path| match HistoryService::spawn(&path) {
            Ok(service) => Some(Arc::new(service)),
            Err(error) => {
                tracing::error!(%error, "cannot open History");
                None
            }
        });

        let bundled = std::env::current_exe()
            .map(|path| onboarding::is_bundled(&path))
            .unwrap_or(false);
        let mut preferences = settings_path()
            .map(|path| Preferences::from_settings(&Settings::load(&path)))
            .unwrap_or_default();
        let language_overridden = args.language.is_some();
        if let Some(language) = args.language {
            preferences.language = language;
        }
        if bundled {
            preferences.launch_at_login = start_at_login(preferences.launch_at_login);
        }

        let (inputs, receiver) = mpsc::channel::<Input>();
        let (finished, controller_done) = mpsc::channel::<()>();
        std::thread::Builder::new()
            .name("utterance-controller".to_owned())
            .spawn({
                let proxy = proxy.clone();
                let inputs = inputs.clone();
                let model = model.clone();
                let history = history.clone();
                move || {
                    if let Err(error) = run_controller(
                        &args,
                        &model,
                        preferences,
                        &proxy,
                        inputs,
                        receiver,
                        history,
                    ) {
                        let _ = proxy.send_event(UserEvent::Fatal(format!("{error:#}")));
                    }
                    let _ = finished.send(());
                }
            })
            .context("starting the controller thread")?;

        if let (Some(path), Some(current)) = (settings_path(), update::current_version()) {
            let update_proxy = proxy.clone();
            if let Err(error) = update::spawn_checker(
                update::Source::github(),
                path,
                current,
                update::FIRST_CHECK_DELAY,
                move |version| {
                    let _ = update_proxy.send_event(UserEvent::Update(
                        version.map(|version| version.to_string()),
                    ));
                },
            ) {
                tracing::warn!(%error, "cannot start the update check");
            }
        }

        let custom_model = model.custom.clone();
        let mut app = App {
            log_dir,
            settings_path: settings_path(),
            menu_bar: None,
            pill: None,
            failure: None,
            language: Language::from_tags(&platform_macos::system_languages()),
            bundled,
            model: ModelStatus::Ready,
            model_dir: model.dir.clone(),
            custom_model,
            model_prompted: false,
            proxy: proxy.clone(),
            inputs,
            report: None,
            setup: None,
            close_setup_at: None,
            history: history.clone(),
            history_window: None,
            preferences,
            language_overridden,
            settings_window: None,
        };
        if model.download {
            app.start_model_download();
        } else if let Some(files) = &model.custom
            && !model_download::missing_files(files).is_empty()
        {
            app.check_custom_model(files);
        }
        event_loop
            .run_app(&mut app)
            .context("running the event loop")?;
        let _ = app.inputs.send(Input::Quit);
        if controller_done.recv_timeout(QUIT_GRACE).is_err() {
            tracing::warn!("the controller did not stop in time; quitting anyway");
        }
        if let Some(history) = &history {
            history.flush();
        }
        platform_macos::restore_clipboard_now();
        match app.failure {
            Some(message) => Err(anyhow::anyhow!(message)),
            None => Ok(()),
        }
    }

    #[derive(Clone)]
    struct ModelSource {
        dir: PathBuf,
        precision: Precision,
        download: bool,
        custom: Option<ModelFiles>,
    }

    impl ModelSource {
        fn choose(args: &Args) -> Self {
            if let Some(dir) = &args.model_dir {
                return Self {
                    dir: dir.clone(),
                    precision: args.precision,
                    download: false,
                    custom: Some(ModelFiles::new(dir, args.precision)),
                };
            }
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default();
            let dir = model_download::default_dir(&home);
            let download = !model_download::is_ready(&dir, &Manifest::pinned());
            Self {
                dir,
                precision: Precision::Int8,
                download,
                custom: None,
            }
        }
    }

    fn lock_path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;
        Some(PathBuf::from(home).join("Library/Application Support/PimSiang/instance.lock"))
    }

    fn settings_path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;
        Some(PathBuf::from(home).join("Library/Application Support/PimSiang/settings"))
    }

    fn history_path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;
        Some(PathBuf::from(home).join("Library/Application Support/PimSiang/history.sqlite"))
    }

    fn start_at_login(wanted: bool) -> bool {
        let first_run =
            settings_path().is_some_and(|path| !Settings::load(&path).login_item_decided());
        if first_run {
            match login_item::set(wanted) {
                Ok(()) => {
                    if let Some(path) = settings_path()
                        && let Err(error) = Settings::update(&path, |settings| {
                            settings.launch_at_login = Some(wanted);
                            settings.login_item_decided = Some(true);
                        })
                    {
                        tracing::warn!(%error, "cannot save the login item choice");
                    }
                }
                Err(reason) => tracing::warn!(%reason, "cannot set the login item"),
            }
        }
        matches!(
            login_item::status(),
            LoginItem::Enabled | LoginItem::RequiresApproval
        )
    }

    fn run_controller(
        args: &Args,
        model: &ModelSource,
        preferences: Preferences,
        proxy: &EventLoopProxy<UserEvent>,
        tx: Sender<Input>,
        rx: mpsc::Receiver<Input>,
        history: Option<Arc<HistoryService>>,
    ) -> Result<()> {
        let status_proxy = proxy.clone();
        let pill_proxy = proxy.clone();
        let permissions_proxy = proxy.clone();
        let sound_proxy = proxy.clone();
        let microphones_proxy = proxy.clone();
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
            Box::new(move |menu| {
                let _ = microphones_proxy.send_event(UserEvent::Microphones(menu));
            }),
        );
        let engine = OnnxEngine::new(ModelFiles::new(&model.dir, model.precision));

        let trigger_tx = tx.clone();
        let audio_tx = tx.clone();
        let lost_tx = tx.clone();
        let loaded_tx = tx;
        let microphone = MicCapture::new(Arc::new(move |samples| {
            let _ = audio_tx.send(Input::Audio(samples));
        }))
        .on_stream_error(Arc::new(move || {
            let _ = lost_tx.send(Input::MicrophoneLost);
        }));

        let mut controller = Controller::new(
            platform,
            engine,
            microphone,
            Config {
                hold_threshold: Duration::from_millis(args.hold_ms),
                ..Config::default()
            },
        )
        .on_model_loaded(move || {
            let _ = loaded_tx.send(Input::ModelLoaded);
        })
        .watch_permissions(PERMISSION_POLL, move || {
            let trigger_tx = trigger_tx.clone();
            Box::new(move |event| {
                let _ = trigger_tx.send(Input::Trigger(event));
            })
        });
        let custom_missing = model
            .custom
            .as_ref()
            .is_some_and(|files| !model_download::missing_files(files).is_empty());
        if model.download || custom_missing {
            controller = controller.model_pending();
        }
        if let Some(history) = history {
            let model_id = model
                .dir
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
            let changed = proxy.clone();
            controller =
                controller.record_history(history_view::recorder(history, model_id, move || {
                    let _ = changed.send_event(UserEvent::HistoryChanged);
                }));
        }
        if let Some(path) = settings_path()
            && let Some(id) = Settings::load(&path).microphone
        {
            controller.handle(Input::Microphone(MicrophoneChoice::Device(id)));
        }
        controller.handle(Input::Preferences(preferences));
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
