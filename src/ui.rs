use crate::{discovery::hostinfo::HostInfo, settings::Configs};
use eframe::{App, NativeOptions};
use egui::{Button, CentralPanel};
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::watch::{Receiver, Sender};

pub(crate) async fn receive_value<T: Clone>(receiver: &mut Receiver<Option<T>>) -> io::Result<T> {
    loop {
        receiver
            .changed()
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "GUI channel closed"))?;
        if let Some(value) = receiver.borrow_and_update().clone() {
            return Ok(value);
        }
    }
}

enum SenderState {
    WaitingForSelectFiles,
    WaitingForUserChoice,
    Sending,
    Completed,
}

enum ReceiverState {
    WaitingForUserChoice,
    Receiving,
    Completed,
}

enum Mode {
    Sender(SenderState),
    Settings,
    Receiver(ReceiverState),
    Unkown,
}

//this enum is simply used to warn the backend
#[derive(Clone)]
pub(crate) enum CommunicationMode {
    Sender,
    Settings,
    Reciever,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Unkown
    }
}

pub(crate) struct Communication {
    //sender prespective
    all_devices_can_send_too: Receiver<Option<Vec<String>>>,
    chosen_reciever: Sender<Option<String>>,
    progress: Receiver<Option<f32>>,
    configs_sender: Sender<Option<Configs>>,
    //reciever prespective
    to_recieve_answer: Sender<Option<bool>>,
    to_recieve: Receiver<Option<HostInfo>>,
    selected_files_to_send: Sender<Option<Vec<PathBuf>>>,
    mode: Sender<Option<CommunicationMode>>,
}

impl Communication {
    pub(crate) fn new(
        all_devices_can_send_too: Receiver<Option<Vec<String>>>,
        chosen_reciever: Sender<Option<String>>,
        progress: Receiver<Option<f32>>,
        to_recieve_answer: Sender<Option<bool>>,
        to_recieve: Receiver<Option<HostInfo>>,
        selected_files_to_send: Sender<Option<Vec<PathBuf>>>,
        mode: Sender<Option<CommunicationMode>>,
        configs_sender: Sender<Option<Configs>>,
    ) -> Self {
        Self {
            chosen_reciever,
            all_devices_can_send_too,
            mode,
            configs_sender,
            progress,
            to_recieve_answer,
            to_recieve,
            selected_files_to_send,
        }
    }
}

/// The backend end of the channels in [`Communication`].
pub(crate) struct BackendCommunication {
    // Sender perspective
    pub(crate) all_devices_can_send_too: Sender<Option<Vec<String>>>,
    pub(crate) chosen_reciever: Receiver<Option<String>>,
    pub(crate) progress: Sender<Option<f32>>,
    // Receiver perspective
    pub(crate) to_recieve_answer: Receiver<Option<bool>>,
    pub(crate) to_recieve: Sender<Option<HostInfo>>,
    pub(crate) selected_files_to_send: Receiver<Option<Vec<PathBuf>>>,
    pub(crate) mode: Receiver<Option<CommunicationMode>>,
    pub(crate) configs: Receiver<Option<Configs>>,
}

impl BackendCommunication {
    pub(crate) fn new(
        all_devices_can_send_too: Sender<Option<Vec<String>>>,
        chosen_reciever: Receiver<Option<String>>,
        progress: Sender<Option<f32>>,
        to_recieve_answer: Receiver<Option<bool>>,
        to_recieve: Sender<Option<HostInfo>>,
        selected_files_to_send: Receiver<Option<Vec<PathBuf>>>,
        mode: Receiver<Option<CommunicationMode>>,
        configs: Receiver<Option<Configs>>,
    ) -> Self {
        Self {
            configs,
            all_devices_can_send_too,
            chosen_reciever,
            progress,
            to_recieve_answer,
            to_recieve,
            selected_files_to_send,
            mode,
        }
    }
}

pub(crate) struct AppData {
    configs: Configs,
    files_to_send: Vec<PathBuf>,
    devices: Vec<String>,
    pending_requests: Option<HostInfo>,
    communication: Communication,
    progress: Option<f32>,
    state: Mode,
}

macro_rules! dots_annimation {
    ($ui:expr) => {{
        let seconds = $ui.input(|i| i.time);
        let dots = (seconds as usize % 3) + 1;
        ".".repeat(dots)
    }};
}

impl App for AppData {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ui, |ui| {
            match &self.state {
                Mode::Unkown => self.choose_mode(ui),
                Mode::Sender(SenderState::WaitingForSelectFiles) => self.select_files(ui),
                Mode::Sender(SenderState::WaitingForUserChoice) => self.choose_device_to_send(ui),
                Mode::Receiver(ReceiverState::WaitingForUserChoice) => self.choose_device_to_recieve_from(ui),
                Mode::Sender(SenderState::Sending) | Mode::Receiver(ReceiverState::Receiving) => self.load_screen(ui),
                Mode::Sender(SenderState::Completed) | Mode::Receiver(ReceiverState::Completed) => self.completed(ui),
                Mode::Settings => self.settings_page(ui),
            };
        });
    }
}

impl AppData {
    pub(crate) fn new(communication: Communication, configs: Configs) -> Self {
        Self {
            configs,
            devices: Vec::new(),
            pending_requests: None,
            files_to_send: Vec::new(),
            progress: None,
            state: Mode::Unkown,
            communication,
        }
    }
}

impl AppData {
    fn choose_mode(&mut self, ui: &mut egui::Ui) {
        let button_width = 150.0;
        let button_height = 65.0;

        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            if ui
                .add_sized([36.0, 36.0], egui::Button::new(egui::RichText::new("⚙").size(22.0)))
                .on_hover_text("Settings")
                .clicked()
            {
                self.state = Mode::Settings;
            }
        });
        ui.vertical_centered(|ui| {
            let stacked = ui.available_width() < button_width * 2.0 + 12.0;

            ui.add_space(ui.available_height() * 0.25);

            ui.heading("Share files effortlessly");
            ui.add_space(6.0);
            ui.label("Send and receive files securely between nearby devices");
            ui.add_space(24.0);

            let mut buttons = |ui: &mut egui::Ui| {
                if ui
                    .add_sized(
                        [button_width, button_height],
                        egui::Button::new(egui::RichText::new("↑  Send Files").size(16.0)),
                    )
                    .clicked()
                {
                    match self.communication.mode.send(Some(CommunicationMode::Sender)) {
                        Ok(()) => {
                            self.state = Mode::Sender(SenderState::WaitingForSelectFiles);
                        },
                        Err(error) => {
                            tracing::error!("Failed to notify backend: {error}");
                        },
                    }
                }

                if ui
                    .add_sized(
                        [button_width, button_height],
                        egui::Button::new(egui::RichText::new("↓  Receive Files").size(16.0)),
                    )
                    .clicked()
                {
                    match self.communication.mode.send(Some(CommunicationMode::Reciever)) {
                        Ok(()) => {
                            self.state = Mode::Receiver(ReceiverState::WaitingForUserChoice);
                        },
                        Err(error) => {
                            tracing::error!("Failed to notify backend: {error}");
                        },
                    }
                }
            };

            if stacked {
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing.y = 12.0;
                    buttons(ui);
                });
            } else {
                let row_width = button_width * 2.0 + 12.0;
                ui.allocate_ui_with_layout(
                    egui::vec2(row_width, button_height),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        buttons(ui);
                    },
                );
            }
        });
    }
    fn choose_device_to_send(&mut self, ui: &mut egui::Ui) {
        if self.communication.all_devices_can_send_too.has_changed().unwrap_or(false)
            && let Some(devices) = self.communication.all_devices_can_send_too.borrow_and_update().clone()
        {
            self.devices = devices;
        }
        let dots = dots_annimation!(ui);

        ui.vertical_centered(|ui| {
            ui.add_space(12.0);
            ui.label(format!("Searching for nearby devices{dots}"));
        });

        for device in &self.devices {
            if ui.button(device).clicked() {
                if let Err(error) = self.communication.chosen_reciever.send(Some(device.clone())) {
                    tracing::warn!("error in device selection: {error}");
                } else {
                    self.state = Mode::Sender(SenderState::Sending);
                }
            }
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
    }
    fn choose_device_to_recieve_from(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            if self.communication.to_recieve.has_changed().unwrap_or(false) {
                self.pending_requests = self.communication.to_recieve.borrow_and_update().clone();
            } else {
                let dots = dots_annimation!(ui);
                ui.add_space(10.0);
                ui.label(format!("waiting on incoming connections{dots}"));
            }

            if let Some(req) = &self.pending_requests {
                ui.add_space(10.0);
                ui.label("Incoming connection");
                ui.add_space(5.0);
                ui.label(format!("{}, wants to send you {} files", req.name(), req.file_num()));

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if ui.button("Accept").clicked() {
                        if let Err(error) = self.communication.to_recieve_answer.send(Some(true)) {
                            tracing::warn!("Failed to send response acceptance: {error}");
                        } else {
                            //no pending requests and advance
                            self.pending_requests = None;
                            self.state = Mode::Receiver(ReceiverState::Receiving);
                        }
                    }
                    ui.add_space(10.0);

                    if ui.button("Reject").clicked() {
                        if let Err(error) = self.communication.to_recieve_answer.send(Some(false)) {
                            tracing::warn!("Failed to send response rejection: {error}");
                        } else {
                            //no pending requests but we still wait
                            self.pending_requests = None;
                        }
                    }
                });
            }
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        });
    }

    fn load_screen(&mut self, ui: &mut egui::Ui) {
        if self.communication.progress.has_changed().unwrap_or(false) {
            self.progress = *self.communication.progress.borrow_and_update();
        }
        if let Some(progress) = &self.progress {
            match self.state {
                Mode::Sender(_) => {
                    ui.vertical_centered(|ui| {
                        let dots = dots_annimation!(ui);

                        ui.label(format!("Sending files{dots}"));
                        ui.add_space(10.0);
                        ui.add(egui::ProgressBar::new(*progress).show_percentage().fill(egui::Color32::GREEN));

                        //concluded
                        if *progress >= 1.0 {
                            self.state = Mode::Sender(SenderState::Completed);
                        }
                    });
                },

                Mode::Receiver(_) => {
                    ui.vertical_centered(|ui| {
                        let dots = dots_annimation!(ui);

                        ui.label(format!("Recieving files{dots}"));
                        ui.add_space(10.0);
                        ui.add(egui::ProgressBar::new(*progress).show_percentage().fill(egui::Color32::GREEN));

                        //concluded
                        if *progress >= 1.0 {
                            self.state = Mode::Receiver(ReceiverState::Completed);
                        }
                    });
                },
                _ => unreachable!(),
            }
        }
    }
    fn select_files(&mut self, ui: &mut egui::Ui) {
        //TODO maybe add a nice screen / annimation
        ui.ctx().input(|i| {
            self.files_to_send
                .extend(i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()));
        });

        let has_files = !self.files_to_send.is_empty();
        let content_height = if has_files { 210.0 } else { 130.0 };
        let top_space = ((ui.available_height() - content_height) / 2.0).max(0.0);

        ui.vertical_centered(|ui| {
            ui.add_space(top_space);
            ui.label(egui::RichText::new("↑").size(48.0));
            ui.add_space(12.0);
            ui.heading("Drop files here");
            ui.label("Drag and drop files from your file manager");

            if has_files {
                ui.add_space(24.0);
                let button = ui.add_sized([140.0, 40.0], Button::new("Send"));
                if button.clicked() {
                    match self
                        .communication
                        .selected_files_to_send
                        .send(Some(std::mem::take(&mut self.files_to_send)))
                    {
                        Ok(_) => self.state = Mode::Sender(SenderState::WaitingForUserChoice),
                        Err(err) => tracing::error!("error sending: {err}"),
                    };
                }
            }
        });
    }
    fn completed(&mut self, ui: &mut egui::Ui) {
        match self.state {
            Mode::Sender(_) => {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.label("All files were sent succesfully");
                });
            },
            Mode::Receiver(_) => {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.label("All files were recieved succesfully");
                });
            },
            _ => unreachable!(),
        }
        ui.vertical_centered(|ui| {
            ui.add_space(10.0);
            if ui.button("Back to home").clicked() {
                self.progress = None;
                self.state = Mode::Unkown;
            }
        });
    }
    fn settings_page(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);

            ui.heading("Settings");
            ui.add_space(20.0);

            ui.label("Download directory");
            ui.add_space(0.0);

            ui.label(self.configs.download_dir.display().to_string());

            ui.add_space(12.0);

            if ui.button("Choose folder").clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .set_title("Choose the folder were the downloaded files will go to")
                    .pick_folder()
            {
                let configs = Configs { download_dir: path };
                match configs.save_config() {
                    Ok(()) => {
                        self.configs = configs.clone();
                        self.communication.configs_sender.send_replace(Some(configs));
                        if let Err(error) = self.communication.mode.send(Some(CommunicationMode::Settings)) {
                            tracing::error!("Failed to notify backend of settings change: {error}");
                        }
                    },
                    Err(error) => tracing::error!("failed to save settings: {error}"),
                }
            }
            ui.add_space(20.0);
            if ui.button("Back").clicked() {
                self.state = Mode::Unkown;
            }
            if ui
                .add(egui::Button::new("Default"))
                .on_hover_text("Reset all settings to default")
                .clicked()
            {
                match Configs::set_default() {
                    Ok(configs) => {
                        self.configs = configs.clone();
                        self.communication.configs_sender.send_replace(Some(configs));
                        if let Err(error) = self.communication.mode.send(Some(CommunicationMode::Settings)) {
                            tracing::error!("Failed to notify backend of settings change: {error}");
                        }
                    },
                    Err(err) => tracing::error!("failed to reset configs to default: {err}"),
                }
            }
        });
    }
}

///The prefered app configs
pub(crate) fn config_eframe() -> NativeOptions {
    let viewport = egui::ViewportBuilder::default()
        .with_drag_and_drop(true)
        .with_title("Drop Sharing")
        .with_resizable(true)
        .with_taskbar(true);

    eframe::NativeOptions {
        viewport,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::receive_value;
    use tokio::sync::watch;

    #[tokio::test]
    async fn receive_value_waits_for_a_new_selection() {
        let (sender, mut receiver) = watch::channel(None);
        sender.send(Some(1)).unwrap();
        assert_eq!(receive_value(&mut receiver).await.unwrap(), 1);

        let task = tokio::spawn(async move { receive_value(&mut receiver).await.unwrap() });
        tokio::task::yield_now().await;
        assert!(!task.is_finished(), "the previous selection was reused");

        sender.send(Some(2)).unwrap();
        assert_eq!(task.await.unwrap(), 2);
    }
}
