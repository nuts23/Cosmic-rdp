use crate::config::Config;
use crate::fl;
use chrono::{DateTime, Utc};
use cosmic::app::context_drawer;
use cosmic::cosmic_config::CosmicConfigEntry;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{self, about::About, icon};
use cosmic_rdp_core::{
    events::{SessionCommand, SessionEvent, SessionState},
    frame::FrameBuffer,
    freerdp::FreeRdpBackend,
    session::{RdpBackend, RdpSessionHandle},
};
use cosmic_rdp_models::{
    devices::{list_local_cameras, CameraDeviceInfo},
    export_rdp_file,
    keyring::SecretStore,
    parse_rdp_file,
    profile::{
        AudioOutputMode, AudioSettings, CameraSettings, CertificatePolicy, ConnectionProfile,
        DisplaySettings, ProfileId, ScalingMode,
    },
    store::ProfileStore,
};
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");
const REPOSITORY: &str = "https://github.com/pop-os/cosmic-rdp";

const AUDIO_MODES: &[&str] = &[
    "Play Locally (High Quality)",
    "Play on Remote Computer",
    "Do Not Play",
];

const SCALING_MODES: &[&str] = &[
    "Dynamic Server Resize (Recommended)",
    "Fit to Window",
    "Original 1:1 Pixel Native",
];

const RESOLUTION_PRESETS: &[&str] = &[
    "1080p Full HD (1920x1080)",
    "2K QHD (2560x1440)",
    "4K UHD (3840x2160)",
    "Ultrawide (3440x1440)",
];

/// Active Drawer / Sheet Page
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextPage {
    About,
    NewConnection,
    EditConnection(ProfileId),
}

/// Active Modal Dialog Type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveDialog {
    ConfirmDelete {
        id: ProfileId,
        name: String,
    },
    VerifyCertificate {
        host: String,
        fingerprint: String,
        common_name: String,
    },
}

/// Form state for adding or editing a connection profile
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileFormState {
    pub id: Option<ProfileId>,
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub domain: String,
    pub password: String,
    pub save_password: bool,
    pub mic_enabled: bool,
    pub audio_output: AudioOutputMode,
    pub low_latency_audio: bool,
    pub camera_enabled: bool,
    pub camera_device: Option<String>,
    pub available_cameras: Vec<CameraDeviceInfo>,
    pub camera_labels: Vec<String>,
    pub dynamic_resolution: bool,
    pub scaling_mode: ScalingMode,
    pub custom_width: u32,
    pub custom_height: u32,
    pub fullscreen: bool,
    pub notes: String,
}

impl Default for ProfileFormState {
    fn default() -> Self {
        let available_cameras = list_local_cameras();
        let mut camera_labels = vec!["Default System Camera".to_string()];
        for cam in &available_cameras {
            camera_labels.push(cam.name.clone());
        }

        Self {
            id: None,
            name: String::new(),
            host: String::new(),
            port: "3389".to_string(),
            username: String::new(),
            domain: String::new(),
            password: String::new(),
            save_password: true,
            mic_enabled: true,
            audio_output: AudioOutputMode::PlayLocally,
            low_latency_audio: true,
            camera_enabled: true,
            camera_device: None,
            available_cameras,
            camera_labels,
            dynamic_resolution: true,
            scaling_mode: ScalingMode::DynamicResize,
            custom_width: 1920,
            custom_height: 1080,
            fullscreen: false,
            notes: String::new(),
        }
    }
}

impl ProfileFormState {
    pub fn from_profile(p: &ConnectionProfile) -> Self {
        let available_cameras = list_local_cameras();
        let mut camera_labels = vec!["Default System Camera".to_string()];
        for cam in &available_cameras {
            camera_labels.push(cam.name.clone());
        }

        Self {
            id: Some(p.id),
            name: p.name.clone(),
            host: p.host.clone(),
            port: p.port.to_string(),
            username: p.username.clone(),
            domain: p.domain.clone().unwrap_or_default(),
            password: String::new(),
            save_password: true,
            mic_enabled: p.audio.microphone_enabled,
            audio_output: p.audio.output_mode,
            low_latency_audio: p.audio.low_latency,
            camera_enabled: p.camera.enabled,
            camera_device: p.camera.device_name.clone(),
            available_cameras,
            camera_labels,
            dynamic_resolution: p.display.dynamic_resolution,
            scaling_mode: p.display.scaling_mode,
            custom_width: p.display.custom_width,
            custom_height: p.display.custom_height,
            fullscreen: p.display.fullscreen,
            notes: p.notes.clone(),
        }
    }

    pub fn to_profile(&self) -> ConnectionProfile {
        let id = self.id.unwrap_or_else(ProfileId::new);
        let port = self.port.parse::<u16>().unwrap_or(3389);
        let domain = if self.domain.trim().is_empty() {
            None
        } else {
            Some(self.domain.trim().to_string())
        };

        ConnectionProfile {
            id,
            name: if self.name.trim().is_empty() {
                self.host.trim().to_string()
            } else {
                self.name.trim().to_string()
            },
            host: self.host.trim().to_string(),
            port,
            username: self.username.trim().to_string(),
            domain,
            cert_fingerprint: None,
            cert_policy: CertificatePolicy::TrustOnFirstUse,
            audio: AudioSettings {
                microphone_enabled: self.mic_enabled,
                output_mode: self.audio_output,
                low_latency: self.low_latency_audio,
                preferred_mic_device: None,
            },
            camera: CameraSettings {
                enabled: self.camera_enabled,
                device_name: self.camera_device.clone(),
            },
            display: DisplaySettings {
                dynamic_resolution: self.dynamic_resolution,
                scaling_mode: self.scaling_mode,
                fullscreen: self.fullscreen,
                custom_width: self.custom_width,
                custom_height: self.custom_height,
                scale_factor: 100,
            },
            color_tag: None,
            last_connected: None,
            notes: self.notes.clone(),
        }
    }
}

/// Active connected RDP session state
pub struct ActiveSessionState {
    pub profile_id: ProfileId,
    pub profile_name: String,
    pub state: SessionState,
    pub server_width: u32,
    pub server_height: u32,
    pub scaling_mode: ScalingMode,
    pub handle: Option<RdpSessionHandle>,
    pub current_frame: Option<FrameBuffer>,
    pub mic_muted: bool,
    pub is_fullscreen: bool,
}

/// Application Model
pub struct AppModel {
    core: cosmic::Core,
    store: ProfileStore,
    search_query: String,
    context_page: Option<ContextPage>,
    active_dialog: Option<ActiveDialog>,
    toast: Option<String>,
    about: About,
    form_state: ProfileFormState,
    active_session: Option<ActiveSessionState>,
    config: Config,
}

/// Messages emitted by widgets, dialogs, and session events
#[derive(Debug, Clone)]
pub enum Message {
    SearchChanged(String),
    OpenNewConnectionDrawer,
    OpenEditConnectionDrawer(ProfileId),
    CloseDrawer,
    UpdateFormName(String),
    UpdateFormHost(String),
    UpdateFormPort(String),
    UpdateFormUsername(String),
    UpdateFormDomain(String),
    UpdateFormPassword(String),
    ToggleFormSavePassword(bool),
    ToggleFormMic(bool),
    ToggleFormLowLatencyAudio(bool),
    SelectAudioOutput(usize),
    ToggleFormCamera(bool),
    SelectCameraDevice(usize),
    ToggleFormDynamicRes(bool),
    SelectScalingMode(usize),
    SelectResolutionPreset(usize),
    ToggleFormFullscreen(bool),
    UpdateFormNotes(String),
    SaveForm,
    DuplicateProfile(ProfileId),
    RequestDeleteProfile(ProfileId, String),
    ConfirmDeleteProfile(ProfileId),
    CancelDialog,
    ImportRdpDialog,
    RdpFileImported(Option<PathBuf>),
    ExportRdpDialog(ProfileId),
    RdpFileExported(Option<PathBuf>, String),
    AcceptCertificate,
    RejectCertificate,
    DismissToast,
    Connect(ProfileId),
    DisconnectSession,
    SessionEvent(SessionEvent),
    ToggleMicMute,
    SendCtrlAltDel,
    ToggleFullscreen,
    LaunchUrl(String),
    ToggleAbout,
    UpdateConfig(Config),
    Noop,
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "dev.cosmic.Rdp";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let store = ProfileStore::load_default().unwrap_or_default();
        let (config, _) = cosmic::cosmic_config::Config::new(Self::APP_ID, crate::config::CONFIG_VERSION)
            .map(|c| (Config::get_entry(&c).unwrap_or_default(), ()))
            .unwrap_or_default();

        let about = About::default()
            .name("Cosmic RDP")
            .icon(icon::from_svg_bytes(APP_ICON))
            .version(env!("CARGO_PKG_VERSION"))
            .author("Christian")
            .comments("Modern, streamlined RDP client optimized for COSMIC & Microsoft Teams")
            .license("GPL-3.0-or-later")
            .links([("Repository", REPOSITORY)]);

        let app = Self {
            core,
            store,
            search_query: String::new(),
            context_page: None,
            active_dialog: None,
            toast: None,
            about,
            form_state: ProfileFormState::default(),
            active_session: None,
            config,
        };

        (app, Task::none())
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::SearchChanged(query) => {
                self.search_query = query;
                Task::none()
            }
            Message::OpenNewConnectionDrawer => {
                self.form_state = ProfileFormState::default();
                self.context_page = Some(ContextPage::NewConnection);
                Task::none()
            }
            Message::OpenEditConnectionDrawer(id) => {
                if let Some(p) = self.store.get(&id) {
                    self.form_state = ProfileFormState::from_profile(p);
                    self.context_page = Some(ContextPage::EditConnection(id));
                }
                Task::none()
            }
            Message::CloseDrawer => {
                self.context_page = None;
                Task::none()
            }
            Message::UpdateFormName(v) => {
                self.form_state.name = v;
                Task::none()
            }
            Message::UpdateFormHost(v) => {
                self.form_state.host = v;
                Task::none()
            }
            Message::UpdateFormPort(v) => {
                self.form_state.port = v;
                Task::none()
            }
            Message::UpdateFormUsername(v) => {
                self.form_state.username = v;
                Task::none()
            }
            Message::UpdateFormDomain(v) => {
                self.form_state.domain = v;
                Task::none()
            }
            Message::UpdateFormPassword(v) => {
                self.form_state.password = v;
                Task::none()
            }
            Message::ToggleFormSavePassword(v) => {
                self.form_state.save_password = v;
                Task::none()
            }
            Message::ToggleFormMic(v) => {
                self.form_state.mic_enabled = v;
                Task::none()
            }
            Message::ToggleFormLowLatencyAudio(v) => {
                self.form_state.low_latency_audio = v;
                Task::none()
            }
            Message::SelectAudioOutput(idx) => {
                self.form_state.audio_output = match idx {
                    0 => AudioOutputMode::PlayLocally,
                    1 => AudioOutputMode::PlayOnRemote,
                    _ => AudioOutputMode::DoNotPlay,
                };
                Task::none()
            }
            Message::ToggleFormCamera(v) => {
                self.form_state.camera_enabled = v;
                Task::none()
            }
            Message::SelectCameraDevice(idx) => {
                if idx == 0 {
                    self.form_state.camera_device = None;
                } else if let Some(cam) = self.form_state.available_cameras.get(idx - 1) {
                    self.form_state.camera_device = Some(cam.path.clone());
                }
                Task::none()
            }
            Message::ToggleFormDynamicRes(v) => {
                self.form_state.dynamic_resolution = v;
                if v {
                    self.form_state.scaling_mode = ScalingMode::DynamicResize;
                }
                Task::none()
            }
            Message::SelectScalingMode(idx) => {
                self.form_state.scaling_mode = match idx {
                    0 => ScalingMode::DynamicResize,
                    1 => ScalingMode::FitWindow,
                    _ => ScalingMode::OriginalSize,
                };
                self.form_state.dynamic_resolution = idx == 0;
                Task::none()
            }
            Message::SelectResolutionPreset(idx) => {
                match idx {
                    0 => {
                        self.form_state.custom_width = 1920;
                        self.form_state.custom_height = 1080;
                    }
                    1 => {
                        self.form_state.custom_width = 2560;
                        self.form_state.custom_height = 1440;
                    }
                    2 => {
                        self.form_state.custom_width = 3840;
                        self.form_state.custom_height = 2160;
                    }
                    3 => {
                        self.form_state.custom_width = 3440;
                        self.form_state.custom_height = 1440;
                    }
                    _ => {}
                }
                Task::none()
            }
            Message::ToggleFormFullscreen(v) => {
                self.form_state.fullscreen = v;
                Task::none()
            }
            Message::UpdateFormNotes(v) => {
                self.form_state.notes = v;
                Task::none()
            }
            Message::SaveForm => {
                let profile = self.form_state.to_profile();
                let pwd = self.form_state.password.clone();
                let save_pwd = self.form_state.save_password;
                let id = profile.id;

                if let Err(err) = self.store.upsert(profile) {
                    error!("Failed to save profile: {:?}", err);
                }

                self.context_page = None;
                self.toast = Some("Saved connection profile".to_string());

                if save_pwd && !pwd.is_empty() {
                    tokio::spawn(async move {
                        if let Err(err) = SecretStore::save_password(id, &pwd).await {
                            warn!("Failed to save password to keyring: {:?}", err);
                        }
                    });
                }
                Task::none()
            }
            Message::DuplicateProfile(id) => {
                if let Some(mut profile) = self.store.get(&id).cloned() {
                    profile.id = ProfileId::new();
                    profile.name = format!("{} (Copy)", profile.name);
                    let new_id = profile.id;
                    if let Err(err) = self.store.upsert(profile) {
                        error!("Failed to duplicate profile: {:?}", err);
                    } else {
                        self.toast = Some("Connection duplicated".to_string());
                        self.form_state = ProfileFormState::from_profile(self.store.get(&new_id).unwrap());
                        self.context_page = Some(ContextPage::EditConnection(new_id));
                    }
                }
                Task::none()
            }
            Message::RequestDeleteProfile(id, name) => {
                self.active_dialog = Some(ActiveDialog::ConfirmDelete { id, name });
                Task::none()
            }
            Message::ConfirmDeleteProfile(id) => {
                if let Err(err) = self.store.remove(&id) {
                    error!("Failed to remove profile {}: {:?}", id, err);
                }
                tokio::spawn(async move {
                    let _ = SecretStore::delete_password(id).await;
                });
                self.active_dialog = None;
                self.context_page = None;
                self.toast = Some("Connection deleted".to_string());
                Task::none()
            }
            Message::CancelDialog => {
                self.active_dialog = None;
                Task::none()
            }
            Message::ImportRdpDialog => {
                Task::perform(
                    async {
                        let file = rfd::AsyncFileDialog::new()
                            .add_filter("RDP Files", &["rdp"])
                            .set_title("Import .rdp File")
                            .pick_file()
                            .await;
                        file.map(|f| f.path().to_path_buf())
                    },
                    |res| cosmic::Action::App(Message::RdpFileImported(res)),
                )
            }
            Message::RdpFileImported(maybe_path) => {
                if let Some(path) = maybe_path {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let hint = path.file_name().and_then(|n| n.to_str());
                        match parse_rdp_file(&content, hint) {
                            Ok(profile) => {
                                let id = profile.id;
                                if let Err(err) = self.store.upsert(profile) {
                                    error!("Failed to save imported profile: {:?}", err);
                                } else {
                                    self.toast = Some(format!("Imported {}", path.file_name().unwrap_or_default().to_string_lossy()));
                                    if let Some(p) = self.store.get(&id) {
                                        self.form_state = ProfileFormState::from_profile(p);
                                        self.context_page = Some(ContextPage::EditConnection(id));
                                    }
                                }
                            }
                            Err(err) => {
                                error!("Failed to parse RDP file: {:?}", err);
                                self.toast = Some(format!("Failed to parse .rdp file: {}", err));
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::ExportRdpDialog(id) => {
                if let Some(profile) = self.store.get(&id) {
                    let rdp_content = export_rdp_file(profile);
                    let default_filename = format!("{}.rdp", profile.name.replace(' ', "_"));
                    Task::perform(
                        async move {
                            let file = rfd::AsyncFileDialog::new()
                                .add_filter("RDP Files", &["rdp"])
                                .set_file_name(&default_filename)
                                .set_title("Export .rdp File")
                                .save_file()
                                .await;
                            (file.map(|f| f.path().to_path_buf()), rdp_content)
                        },
                        |(res, content)| cosmic::Action::App(Message::RdpFileExported(res, content)),
                    )
                } else {
                    Task::none()
                }
            }
            Message::RdpFileExported(maybe_path, content) => {
                if let Some(path) = maybe_path {
                    if let Err(err) = std::fs::write(&path, content) {
                        error!("Failed to export RDP file: {:?}", err);
                        self.toast = Some(format!("Failed to export file: {}", err));
                    } else {
                        self.toast = Some("Successfully exported .rdp file".to_string());
                    }
                }
                Task::none()
            }
            Message::AcceptCertificate => {
                self.active_dialog = None;
                self.toast = Some("Certificate trusted and connection resumed".to_string());
                Task::none()
            }
            Message::RejectCertificate => {
                self.active_dialog = None;
                if let Some(ref session) = self.active_session {
                    if let Some(ref handle) = session.handle {
                        let h = handle.clone();
                        tokio::spawn(async move {
                            let _ = h.disconnect().await;
                        });
                    }
                }
                self.active_session = None;
                Task::none()
            }
            Message::DismissToast => {
                self.toast = None;
                Task::none()
            }
            Message::Connect(id) => {
                if let Some(mut profile) = self.store.get(&id).cloned() {
                    info!("Initiating connection to profile: {} ({}:{})", profile.name, profile.host, profile.port);
                    profile.last_connected = Some(Utc::now());
                    let _ = self.store.upsert(profile.clone());

                    let (cmd_tx, cmd_rx) = mpsc::channel(64);
                    let (event_tx, event_rx) = mpsc::channel(64);
                    let handle = RdpSessionHandle::new(cmd_tx);

                    let initial_w = if profile.display.dynamic_resolution { 1920 } else { profile.display.custom_width };
                    let initial_h = if profile.display.dynamic_resolution { 1080 } else { profile.display.custom_height };

                    self.active_session = Some(ActiveSessionState {
                        profile_id: id,
                        profile_name: profile.name.clone(),
                        state: SessionState::Connecting {
                            message: format!("Connecting to {} ({}:{})...", profile.name, profile.host, profile.port),
                        },
                        server_width: initial_w,
                        server_height: initial_h,
                        scaling_mode: profile.display.scaling_mode,
                        handle: Some(handle),
                        current_frame: None,
                        mic_muted: false,
                        is_fullscreen: profile.display.fullscreen,
                    });

                    // Start background FreeRDP engine runner
                    tokio::spawn(async move {
                        let mut backend = FreeRdpBackend::new();
                        let pwd = SecretStore::get_password(id).await.ok();
                        if let Err(err) = backend.start(profile, pwd, event_tx, cmd_rx).await {
                            error!("FreeRDP session error: {:?}", err);
                        }
                    });

                    // Stream to forward events directly to the update loop
                    return Task::run(
                        futures::stream::unfold(event_rx, |mut rx| async move {
                            rx.recv().await.map(|evt| (Message::SessionEvent(evt), rx))
                        }),
                        cosmic::Action::App,
                    );
                }
                Task::none()
            }
            Message::SessionEvent(evt) => {
                info!("SessionEvent received in app: {:?}", evt);
                match evt {
                    SessionEvent::StateChanged(st) => {
                        info!("Session state changed: {:?}", st);
                        if let Some(ref mut session) = self.active_session {
                            session.state = st;
                        }
                    }
                    SessionEvent::FrameUpdate(frame) => {
                        if let Some(ref mut session) = self.active_session {
                            session.server_width = frame.width;
                            session.server_height = frame.height;
                            session.current_frame = Some(frame);
                        }
                    }
                    SessionEvent::ServerResolutionChanged { width, height } => {
                        info!("Remote resolution updated: {}x{}", width, height);
                        if let Some(ref mut session) = self.active_session {
                            session.server_width = width;
                            session.server_height = height;
                        }
                    }
                    SessionEvent::CertificatePrompt { host, fingerprint, common_name } => {
                        self.active_dialog = Some(ActiveDialog::VerifyCertificate {
                            host,
                            fingerprint,
                            common_name,
                        });
                    }
                    SessionEvent::Error(err) => {
                        error!("Session error received: {}", err);
                        if let Some(ref mut session) = self.active_session {
                            session.state = SessionState::Failed { reason: err };
                        }
                    }
                    _ => {}
                }
                Task::none()
            }
            Message::DisconnectSession => {
                if let Some(ref session) = self.active_session {
                    if let Some(ref handle) = session.handle {
                        let h = handle.clone();
                        tokio::spawn(async move {
                            let _ = h.disconnect().await;
                        });
                    }
                }
                self.active_session = None;
                Task::none()
            }
            Message::ToggleMicMute => {
                if let Some(ref mut session) = self.active_session {
                    session.mic_muted = !session.mic_muted;
                    if let Some(ref handle) = session.handle {
                        let h = handle.clone();
                        let muted = session.mic_muted;
                        tokio::spawn(async move {
                            let _ = h.send_command(SessionCommand::MuteMicrophone(muted)).await;
                        });
                    }
                }
                Task::none()
            }
            Message::SendCtrlAltDel => {
                if let Some(ref session) = self.active_session {
                    if let Some(ref handle) = session.handle {
                        let h = handle.clone();
                        tokio::spawn(async move {
                            let _ = h.send_command(SessionCommand::SendCtrlAltDel).await;
                        });
                    }
                }
                Task::none()
            }
            Message::ToggleFullscreen => {
                if let Some(ref mut session) = self.active_session {
                    session.is_fullscreen = !session.is_fullscreen;
                }
                Task::none()
            }
            Message::LaunchUrl(url) => {
                let _ = open::that_detached(url);
                Task::none()
            }
            Message::ToggleAbout => {
                if self.context_page == Some(ContextPage::About) {
                    self.context_page = None;
                } else {
                    self.context_page = Some(ContextPage::About);
                }
                Task::none()
            }
            Message::UpdateConfig(cfg) => {
                self.config = cfg;
                Task::none()
            }
            Message::Noop => Task::none(),
        }
    }

    fn on_window_resize(&mut self, _id: cosmic::iced::window::Id, width: f32, height: f32) {
        if let Some(ref mut session) = self.active_session {
            if session.scaling_mode == ScalingMode::DynamicResize {
                let w = width.max(640.0) as u32;
                let h = height.max(480.0) as u32;
                session.server_width = w;
                session.server_height = h;
                if let Some(ref handle) = session.handle {
                    let h_clone = handle.clone();
                    tokio::spawn(async move {
                        let _ = h_clone
                            .send_command(SessionCommand::Resize {
                                width: w,
                                height: h,
                                scale_factor: 100,
                            })
                            .await;
                    });
                }
            }
        }
    }

    fn subscription(&self) -> cosmic::iced::Subscription<Self::Message> {
        if self.active_session.is_some() {
            cosmic::iced::event::listen_with(|event, _status, _window| {
                if let cosmic::iced::Event::Keyboard(cosmic::iced::keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    ..
                }) = event
                {
                    if modifiers.control() && modifiers.alt() && modifiers.shift() {
                        match key {
                            cosmic::iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("f") => {
                                Some(Message::ToggleFullscreen)
                            }
                            cosmic::iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("m") => {
                                Some(Message::ToggleMicMute)
                            }
                            cosmic::iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("d") => {
                                Some(Message::DisconnectSession)
                            }
                            cosmic::iced::keyboard::Key::Named(cosmic::iced::keyboard::key::Named::End) => {
                                Some(Message::SendCtrlAltDel)
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
        } else {
            cosmic::iced::Subscription::none()
        }
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        let mut elements = Vec::new();
        if self.active_session.is_none() {
            let add_btn = widget::button::icon(icon::from_name("list-add-symbolic"))
                .on_press(Message::OpenNewConnectionDrawer)
                .tooltip(fl!("new-connection"));
            let import_btn = widget::button::icon(icon::from_name("document-open-symbolic"))
                .on_press(Message::ImportRdpDialog)
                .tooltip(fl!("import-rdp"));

            elements.push(add_btn.into());
            elements.push(import_btn.into());
        }
        elements
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        let mut elements = Vec::new();
        let about_btn = widget::button::icon(icon::from_name("help-about-symbolic"))
            .on_press(Message::ToggleAbout)
            .tooltip("About Cosmic RDP");
        elements.push(about_btn.into());
        elements
    }

    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Self::Message>> {
        let page = self.context_page?;
        let content: Element<'_, Self::Message> = match page {
            ContextPage::About => widget::about::about(&self.about, |url| Message::LaunchUrl(url.to_string())),
            ContextPage::NewConnection => self.view_profile_drawer(true),
            ContextPage::EditConnection(_) => self.view_profile_drawer(false),
        };

        let title = match page {
            ContextPage::About => fl!("about"),
            ContextPage::NewConnection => fl!("new-connection"),
            ContextPage::EditConnection(_) => fl!("edit-connection"),
        };

        Some(
            context_drawer::context_drawer(content, Message::CloseDrawer)
                .title(title),
        )
    }

    fn dialog(&self) -> Option<Element<'_, Self::Message>> {
        let dialog = self.active_dialog.as_ref()?;
        match dialog {
            ActiveDialog::ConfirmDelete { id, name } => {
                let id_clone = *id;
                let dlg = widget::dialog()
                    .title(fl!("confirm-delete-title"))
                    .body(fl!("confirm-delete-body", name = name.as_str()))
                    .icon(widget::icon::from_name("dialog-warning-symbolic").size(32))
                    .primary_action(
                        widget::button::destructive(fl!("delete"))
                            .on_press(Message::ConfirmDeleteProfile(id_clone)),
                    )
                    .secondary_action(
                        widget::button::standard(fl!("cancel"))
                            .on_press(Message::CancelDialog),
                    );
                Some(dlg.into())
            }
            ActiveDialog::VerifyCertificate { host, fingerprint, common_name } => {
                let body_text = format!("Host: {}\nCommon Name: {}\nSHA-256 Fingerprint:\n{}", host, common_name, fingerprint);
                let dlg = widget::dialog()
                    .title(fl!("cert-verify-title"))
                    .body(body_text)
                    .icon(widget::icon::from_name("channel-insecure-symbolic").size(32))
                    .primary_action(
                        widget::button::suggested(fl!("cert-accept"))
                            .on_press(Message::AcceptCertificate),
                    )
                    .secondary_action(
                        widget::button::destructive(fl!("cert-reject"))
                            .on_press(Message::RejectCertificate),
                    );
                Some(dlg.into())
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let main_view: Element<'_, Self::Message> = if let Some(ref session) = self.active_session {
            self.view_active_session(session)
        } else {
            self.view_hub()
        };

        // If toast notification exists, render it floating at bottom
        if let Some(ref toast_text) = self.toast {
            let toast_bar = widget::container(
                widget::row::with_capacity(3)
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .push(widget::icon::from_name("emblem-ok-symbolic").size(18))
                    .push(widget::text::body(toast_text.clone()))
                    .push(
                        widget::button::icon(icon::from_name("window-close-symbolic"))
                            .on_press(Message::DismissToast),
                    ),
            )
            .padding([8, 16]);

            widget::column::with_capacity(2)
                .push(widget::container(main_view).height(Length::Fill))
                .push(
                    widget::container(toast_bar)
                        .width(Length::Fill)
                        .align_x(Horizontal::Center),
                )
                .into()
        } else {
            main_view
        }
    }
}

impl AppModel {
    /// Render Connection Hub (grid / cards of target Windows machines)
    fn view_hub(&self) -> Element<'_, Message> {
        let profiles = self.store.list();
        let filtered: Vec<_> = profiles
            .into_iter()
            .filter(|p| {
                if self.search_query.is_empty() {
                    true
                } else {
                    p.name.to_lowercase().contains(&self.search_query.to_lowercase())
                        || p.host.to_lowercase().contains(&self.search_query.to_lowercase())
                        || p.username.to_lowercase().contains(&self.search_query.to_lowercase())
                }
            })
            .collect();

        // Search Bar & Import Action
        let search_input = widget::text_input(
            fl!("search-placeholder"),
            &self.search_query,
        )
        .on_input(Message::SearchChanged)
        .leading_icon(widget::icon::from_name("system-search-symbolic").into())
        .width(Length::Fill);

        let top_bar = widget::row::with_capacity(3)
            .spacing(12)
            .align_y(Alignment::Center)
            .push(search_input)
            .push(
                widget::button::standard(fl!("import-rdp"))
                    .on_press(Message::ImportRdpDialog),
            );

        let content: Element<'_, Message> = if filtered.is_empty() {
            widget::column::with_capacity(4)
                .spacing(16)
                .align_x(Alignment::Center)
                .push(widget::icon::from_name("network-server-symbolic").size(64))
                .push(widget::text::title3(fl!("no-connections")))
                .push(
                    widget::row::with_capacity(2)
                        .spacing(12)
                        .push(
                            widget::button::suggested(fl!("new-connection"))
                                .on_press(Message::OpenNewConnectionDrawer),
                        )
                        .push(
                            widget::button::standard(fl!("import-rdp"))
                                .on_press(Message::ImportRdpDialog),
                        ),
                )
                .into()
        } else {
            let mut cards_col = widget::column::with_capacity(filtered.len())
                .spacing(12)
                .width(Length::Fill);
            for p in filtered {
                cards_col = cards_col.push(Self::view_profile_card(&p));
            }
            widget::scrollable(cards_col).into()
        };

        widget::container(
            widget::column::with_capacity(2)
                .spacing(16)
                .push(top_bar)
                .push(content),
        )
        .padding(16)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    /// Render individual connection profile card
    fn view_profile_card<'a>(p: &ConnectionProfile) -> Element<'a, Message> {
        let id = p.id;
        let host_label = format!("{}:{}", p.host, p.port);
        let user_label = if p.username.is_empty() {
            "No username".to_string()
        } else if let Some(ref d) = p.domain {
            format!("{}\\{}", d, p.username)
        } else {
            p.username.clone()
        };

        let last_conn_text = format_last_connected(p.last_connected);

        // Badges for active Teams optimizations
        let mut badges = widget::row::with_capacity(4).spacing(6);
        if p.audio.microphone_enabled {
            badges = badges.push(
                widget::container(
                    widget::row::with_capacity(2)
                        .spacing(4)
                        .align_y(Alignment::Center)
                        .push(widget::icon::from_name("audio-input-microphone-symbolic").size(14))
                        .push(widget::text::caption("Mic")),
                )
                .padding([2, 6]),
            );
        }
        if p.camera.enabled {
            badges = badges.push(
                widget::container(
                    widget::row::with_capacity(2)
                        .spacing(4)
                        .align_y(Alignment::Center)
                        .push(widget::icon::from_name("camera-web-symbolic").size(14))
                        .push(widget::text::caption("Cam")),
                )
                .padding([2, 6]),
            );
        }
        if p.display.dynamic_resolution {
            badges = badges.push(
                widget::container(
                    widget::row::with_capacity(2)
                        .spacing(4)
                        .align_y(Alignment::Center)
                        .push(widget::icon::from_name("video-display-symbolic").size(14))
                        .push(widget::text::caption("Dynamic Res")),
                )
                .padding([2, 6]),
            );
        }

        let info_col = widget::column::with_capacity(4)
            .spacing(4)
            .push(widget::text::title3(p.name.clone()))
            .push(
                widget::row::with_capacity(3)
                    .spacing(12)
                    .push(widget::text::body(host_label))
                    .push(widget::text::caption(user_label))
                    .push(widget::text::caption(last_conn_text)),
            )
            .push(badges);

        let actions = widget::row::with_capacity(4)
            .spacing(8)
            .align_y(Alignment::Center)
            .push(
                widget::button::icon(icon::from_name("edit-copy-symbolic"))
                    .on_press(Message::DuplicateProfile(id))
                    .tooltip(fl!("duplicate")),
            )
            .push(
                widget::button::icon(icon::from_name("document-edit-symbolic"))
                    .on_press(Message::OpenEditConnectionDrawer(id))
                    .tooltip(fl!("edit-connection")),
            )
            .push(
                widget::button::suggested(fl!("connect"))
                    .on_press(Message::Connect(id)),
            );

        widget::container(
            widget::row::with_capacity(3)
                .spacing(16)
                .align_y(Alignment::Center)
                .push(widget::icon::from_name("computer-symbolic").size(36))
                .push(widget::container(info_col).width(Length::Fill))
                .push(actions),
        )
        .padding(14)
        .width(Length::Fill)
        .into()
    }

    /// Render streamlined settings drawer for creating or editing a profile
    fn view_profile_drawer<'a>(&'a self, is_new: bool) -> Element<'a, Message> {
        let f = &self.form_state;

        let name_input = widget::text_input("Friendly Name (e.g. Work PC)", &f.name)
            .on_input(Message::UpdateFormName);

        let host_input = widget::text_input("Hostname or IP address", &f.host)
            .on_input(Message::UpdateFormHost);

        let port_input = widget::text_input("Port (default 3389)", &f.port)
            .on_input(Message::UpdateFormPort);

        let user_input = widget::text_input("Username", &f.username)
            .on_input(Message::UpdateFormUsername);

        let domain_input = widget::text_input("Domain (optional)", &f.domain)
            .on_input(Message::UpdateFormDomain);

        let pass_input = widget::text_input("Password (optional, saved to Keyring)", &f.password)
            .on_input(Message::UpdateFormPassword)
            .password();

        let save_pass_toggle = widget::toggler(f.save_password)
            .label(fl!("save-password"))
            .on_toggle(Message::ToggleFormSavePassword);

        // Teams Audio Redirection Options
        let mic_toggle = widget::toggler(f.mic_enabled)
            .label(fl!("microphone"))
            .on_toggle(Message::ToggleFormMic);

        let low_latency_toggle = widget::toggler(f.low_latency_audio)
            .label(fl!("low-latency-audio"))
            .on_toggle(Message::ToggleFormLowLatencyAudio);

        let audio_idx = match f.audio_output {
            AudioOutputMode::PlayLocally => 0,
            AudioOutputMode::PlayOnRemote => 1,
            AudioOutputMode::DoNotPlay => 2,
        };
        let audio_dropdown = widget::dropdown(
            AUDIO_MODES,
            Some(audio_idx),
            Message::SelectAudioOutput,
        );

        // Teams Camera Redirection Options
        let camera_toggle = widget::toggler(f.camera_enabled)
            .label(fl!("camera"))
            .on_toggle(Message::ToggleFormCamera);

        let selected_cam_idx = f
            .camera_device
            .as_ref()
            .and_then(|dev| f.available_cameras.iter().position(|c| &c.path == dev))
            .map(|pos| pos + 1)
            .unwrap_or(0);

        let camera_dropdown = widget::dropdown(
            &f.camera_labels[..],
            Some(selected_cam_idx),
            Message::SelectCameraDevice,
        );

        // Display & Resolution Options
        let dynamic_res_toggle = widget::toggler(f.dynamic_resolution)
            .label(fl!("dynamic-resolution"))
            .on_toggle(Message::ToggleFormDynamicRes);

        let scaling_idx = match f.scaling_mode {
            ScalingMode::DynamicResize => 0,
            ScalingMode::FitWindow => 1,
            ScalingMode::OriginalSize => 2,
        };
        let scaling_dropdown = widget::dropdown(
            SCALING_MODES,
            Some(scaling_idx),
            Message::SelectScalingMode,
        );

        let res_idx = match (f.custom_width, f.custom_height) {
            (2560, 1440) => 1,
            (3840, 2160) => 2,
            (3440, 1440) => 3,
            _ => 0,
        };
        let res_dropdown = widget::dropdown(
            RESOLUTION_PRESETS,
            Some(res_idx),
            Message::SelectResolutionPreset,
        );

        let fullscreen_toggle = widget::toggler(f.fullscreen)
            .label(fl!("fullscreen"))
            .on_toggle(Message::ToggleFormFullscreen);

        let notes_input = widget::text_input("Notes / Remarks", &f.notes)
            .on_input(Message::UpdateFormNotes);

        let mut btn_row = widget::row::with_capacity(4)
            .spacing(12)
            .align_y(Alignment::Center)
            .push(
                widget::button::suggested(fl!("save"))
                    .on_press(Message::SaveForm),
            )
            .push(
                widget::button::standard(fl!("cancel"))
                    .on_press(Message::CloseDrawer),
            );

        if !is_new {
            if let Some(id) = f.id {
                btn_row = btn_row
                    .push(
                        widget::button::standard(fl!("export-rdp"))
                            .on_press(Message::ExportRdpDialog(id)),
                    )
                    .push(
                        widget::button::destructive(fl!("delete-connection"))
                            .on_press(Message::RequestDeleteProfile(id, f.name.clone())),
                    );
            }
        }

        let form = widget::column::with_capacity(20)
            .spacing(14)
            .push(widget::text::title4("General"))
            .push(name_input)
            .push(host_input)
            .push(port_input)
            .push(widget::text::title4("Credentials"))
            .push(user_input)
            .push(domain_input)
            .push(pass_input)
            .push(save_pass_toggle)
            .push(widget::text::title4("Teams & Audio Redirection"))
            .push(mic_toggle)
            .push(low_latency_toggle)
            .push(widget::text::caption(fl!("audio-output")))
            .push(audio_dropdown)
            .push(widget::text::title4("Camera Passthrough"))
            .push(camera_toggle)
            .push(widget::text::caption(fl!("camera-device")))
            .push(camera_dropdown)
            .push(widget::text::title4("Display & Resolution"))
            .push(dynamic_res_toggle)
            .push(widget::text::caption(fl!("scaling-mode")))
            .push(scaling_dropdown)
            .push(widget::text::caption(fl!("resolution-preset")))
            .push(res_dropdown)
            .push(fullscreen_toggle)
            .push(notes_input)
            .push(btn_row);

        widget::scrollable(form).into()
    }

    /// Render active RDP session viewport with top floating toolbar
    fn view_active_session<'a>(&'a self, session: &'a ActiveSessionState) -> Element<'a, Message> {
        // Floating Top Toolbar
        let mic_icon_name = if session.mic_muted {
            "microphone-disabled-symbolic"
        } else {
            "audio-input-microphone-symbolic"
        };

        let res_badge = format!("{}x{}", session.server_width, session.server_height);

        let floating_bar = widget::container(
            widget::row::with_capacity(6)
                .spacing(12)
                .align_y(Alignment::Center)
                .push(widget::icon::from_name("computer-symbolic").size(20))
                .push(widget::text::title4(&session.profile_name))
                .push(
                    widget::container(widget::text::caption(res_badge))
                        .padding([2, 6]),
                )
                .push(
                    widget::button::icon(icon::from_name(mic_icon_name))
                        .on_press(Message::ToggleMicMute)
                        .tooltip("Toggle Mic Mute"),
                )
                .push(
                    widget::button::icon(icon::from_name("view-fullscreen-symbolic"))
                        .on_press(Message::ToggleFullscreen)
                        .tooltip(fl!("fullscreen")),
                )
                .push(
                    widget::button::icon(icon::from_name("system-lock-screen-symbolic"))
                        .on_press(Message::SendCtrlAltDel)
                        .tooltip("Send Ctrl+Alt+Del"),
                )
                .push(
                    widget::button::destructive(fl!("disconnect"))
                        .on_press(Message::DisconnectSession),
                ),
        )
        .padding([6, 16]);

        // Main Desktop Frame or Connecting Spinner
        let session_content: Element<'a, Message> = match &session.state {
            SessionState::Connecting { message } => widget::column::with_capacity(3)
                .spacing(16)
                .align_x(Alignment::Center)
                .push(widget::icon::from_name("network-transmit-receive-symbolic").size(48))
                .push(widget::text::title3(message.as_str()))
                .push(
                    widget::button::standard("Cancel")
                        .on_press(Message::DisconnectSession),
                )
                .into(),
            SessionState::Failed { reason } => widget::column::with_capacity(3)
                .spacing(16)
                .align_x(Alignment::Center)
                .push(widget::icon::from_name("dialog-error-symbolic").size(48))
                .push(widget::text::title3(format!("Connection Failed: {}", reason)))
                .push(
                    widget::button::standard("Back to Hub")
                        .on_press(Message::DisconnectSession),
                )
                .into(),
            SessionState::Connected | SessionState::Reconnecting { .. } => {
                if let Some(ref frame) = session.current_frame {
                    let desc = format!("Active Session ({}x{}) - Remote Connected", frame.width, frame.height);
                    widget::column::with_capacity(1)
                        .spacing(12)
                        .align_x(Alignment::Center)
                        .push(widget::text::caption(desc))
                        .into()
                } else {
                    widget::text::title3("Session Connected").into()
                }
            }
            SessionState::Disconnected => widget::column::with_capacity(2)
                .spacing(16)
                .align_x(Alignment::Center)
                .push(widget::text::title3(fl!("status-disconnected")))
                .push(
                    widget::button::standard("Back to Hub")
                        .on_press(Message::DisconnectSession),
                )
                .into(),
        };

        widget::column::with_capacity(2)
            .spacing(8)
            .push(
                widget::container(floating_bar)
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
            )
            .push(
                widget::container(session_content)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center),
            )
            .into()
    }
}

fn format_last_connected(last: Option<DateTime<Utc>>) -> String {
    if let Some(time) = last {
        let diff = Utc::now().signed_duration_since(time);
        if diff.num_minutes() < 1 {
            "Just now".to_string()
        } else if diff.num_minutes() < 60 {
            format!("{} min ago", diff.num_minutes())
        } else if diff.num_hours() < 24 {
            format!("{} hr ago", diff.num_hours())
        } else {
            format!("{} days ago", diff.num_days())
        }
    } else {
        "Never connected".to_string()
    }
}
