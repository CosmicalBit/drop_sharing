use crate::discovery::hostinfo::HostInfo;
use eframe::App;
use egui::CentralPanel;
use egui_material_icons::icons;
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::watch::{Receiver, Sender};

pub(crate) async fn receive_value<T: Clone>(receiver: &mut Receiver<Option<T>>) -> io::Result<T> {
    loop {
        if let Some(value) = receiver.borrow_and_update().clone() {
            return Ok(value);
        }
        receiver
            .changed()
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "GUI channel closed"))?;
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
    Receiver(ReceiverState),
    Unkown,
}

//this enum is simply used to warn the backend
#[derive(Clone)]
pub(crate) enum CommunicationMode {
    Sender,
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
    ) -> Self {
        Self {
            all_devices_can_send_too,
            mode,
            chosen_reciever,
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
    ) -> Self {
        Self {
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
            };
        });
    }
}

impl AppData {
    pub(crate) fn new(communication: Communication) -> Self {
        Self {
            devices: Vec::new(),
            pending_requests: None,
            progress: None,
            state: Mode::Unkown,
            communication,
        }
    }
}

impl AppData {
    fn choose_mode(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(12.0);

            ui.label("Share files with nearby devices");
            ui.add_space(30.0);

            if ui.button("Sender").clicked() {
                if let Err(error) = self.communication.mode.send(Some(CommunicationMode::Sender)) {
                    tracing::error!("failed sending mode to backend: {error}");
                };
                self.state = Mode::Sender(SenderState::WaitingForSelectFiles);
            }

            ui.add_space(8.0);

            if ui.button("Reciever").clicked() {
                if let Err(error) = self.communication.mode.send(Some(CommunicationMode::Reciever)) {
                    tracing::error!("failed sending mode to backend: {error}");
                };
                self.state = Mode::Receiver(ReceiverState::WaitingForUserChoice);
            }
        });
    }

    fn choose_device_to_send(&mut self, ui: &mut egui::Ui) {
        if self.communication.all_devices_can_send_too.has_changed().unwrap_or(false) {
            if let Some(devices) = self.communication.all_devices_can_send_too.borrow_and_update().clone() {
                self.devices = devices;
            }
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
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new(icons::ICON_UPLOAD_2).size(48.0));
            ui.heading("Drop files here");
            ui.label("Drag and drop files from your file manager");
        });

        let files: Vec<PathBuf> = ui
            .ctx()
            .input(|i| i.raw.dropped_files.iter().map(|file| file.path().to_path_buf()).collect());

        //TODO maybe add a nice screen / annimation
        if !files.is_empty() {
            if let Err(err) = self.communication.selected_files_to_send.send(Some(files)) {
                tracing::error!("impossible to select files : {err}");
            } else {
                self.state = Mode::Sender(SenderState::WaitingForUserChoice);
            }
        }
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
        // FIXME go back to homescreen instead or mayb this but pretty
        self.state = Mode::Unkown;
    }
}
