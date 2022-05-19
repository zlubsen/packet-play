use std::process::exit;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::Duration;
use eframe::NativeOptions;
use egui::Button;
use log::{error, trace};
use crate::{Cli};
use crate::constants::{ERROR_CREATE_PLAYER, ERROR_INIT_PLAYER, ERROR_INIT_PLAYER_TIMEOUT, PLAYER_STARTUP_TIMEOUT_MS};
use crate::model::pcap::Pcap;
use crate::model::{Command, Event, PositionChange, Recording};
use crate::player::{Player, PlayerState};

pub(crate) fn run_gui(cli : Cli, recording: Pcap) {
    let (cmd_sender, cmd_receiver) = mpsc::channel();
    let (event_sender, event_receiver) = mpsc::channel();

    // TODO handle errors on creation of player
    // Spawn thread for the Player
    let _player_handle = match Player::builder()
        .recording(Recording::PCAP(recording))
        .destination(cli.destination)
        .source_port(cli.source_port)
        .ttl(cli.ttl)
        .cmd_rx(cmd_receiver)
        .event_tx(event_sender)
        .build() {
        Ok(handle) => { handle }
        Err(err) => { error!("{err:?}"); exit(ERROR_CREATE_PLAYER); }
    };
    // Wait for Player to be initialised
    loop {
        match event_receiver.recv_timeout(Duration::from_secs(PLAYER_STARTUP_TIMEOUT_MS)) {
            Ok(event) => {
                match event {
                    Event::Error(_) => { exit(ERROR_INIT_PLAYER) }
                    Event::PlayerReady => {
                        break; }
                    _ => { trace!("Unexpected to see this event here..."); }
                }
            }
            Err(_) => {
                exit(ERROR_INIT_PLAYER_TIMEOUT)
            }
        }
    }

    if !cli.auto_play_disable {
        let _ = cmd_sender.send(Command::Play);
    }

    let options = window_options();
    eframe::run_native(
        "packet-play",
        options,
        Box::new(|_cc| Box::new(
            GuiApp::new(cli, cmd_sender, event_receiver)))
    );
}

struct GuiApp {
    cli: Cli,
    current_state : PlayerState,
    current_position : PositionChange,
    cmd_sender: Sender<Command>,
    event_receiver: Receiver<Event>,
}

impl GuiApp {
    pub fn new(cli: Cli, cmd_sender: Sender<Command>, event_receiver: Receiver<Event>) -> Self {
        Self {
            cli,
            current_state: PlayerState::Initial,
            current_position: Default::default(),
            cmd_sender,
            event_receiver,
        }
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let message : Option<String> = match self.event_receiver.try_recv() {
            Ok(Event::QuitCommanded) => { None } // Not needed with GUI
            Ok(Event::PlayerReady) => { None } // Already passed
            Ok(Event::PlayerStateChanged(state)) => {
                self.current_state = state.state;
                None
            }
            Ok(Event::PlayerPositionChanged(position)) => {
                self.current_position = position;
                None
            }
            Ok(Event::Error(error)) => { Some(format!("{error:?}")) }
            Err(TryRecvError::Empty) => { None }
            Err(TryRecvError::Disconnected) => {
                Some("Event channel disconnected, Player stopped working. Exiting.".to_string())
            }
        };

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Packet Play");
            ui.label(format!("Recording: {}", self.cli.file));

            ui.horizontal(|ui| {
                ui.label(self.current_state.to_string());
                ui.add(egui::ProgressBar::new(
                    self.current_position.position as f32 / self.current_position.max_position as f32)
                    .animate(self.current_state == PlayerState::Playing));

            });
            ui.label(format!("Packets: [{}/{}]", self.current_position.position, self.current_position.max_position));
            ui.label(format!("Time: [ {} / {} ]", indicatif::FormattedDuration(self.current_position.time_position), indicatif::FormattedDuration(self.current_position.time_total)));

            ui.horizontal(|ui| {
                if ui.add_enabled(
                    self.current_state != PlayerState::Playing,
                    Button::new("Play")).clicked() {
                    let _ = self.cmd_sender.send(Command::Play);
                }
                if ui.add_enabled(
                    self.current_state == PlayerState::Playing,
                    Button::new("Pause")).clicked() {
                    let _ = self.cmd_sender.send(Command::Pause);
                }
                if ui.add_enabled(
                    self.current_state != PlayerState::Initial,
                    Button::new("Rewind")).clicked() {
                    let _ = self.cmd_sender.send(Command::Rewind);
                }
            });
            ui.label(message.unwrap_or("".to_string()));
        });
    }

    fn on_exit(&mut self, _gl: &eframe::glow::Context) {
        // Send Quit command to Player
        let _ = self.cmd_sender.send(Command::Quit);
        // Wait for player to shutdown
        loop {
            if let Ok(Event::QuitCommanded) = self.event_receiver.try_recv() {
                break;
            }
        }
    }
}

fn window_options() -> NativeOptions {
    NativeOptions {
        decorated: true,
        initial_window_size: Some(egui::Vec2::new(500f32,150f32)),
        ..Default::default()
    }
}