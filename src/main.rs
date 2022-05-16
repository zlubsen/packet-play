mod model;
mod player;
mod constants;

use std::{env, thread};
use std::fs::File;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::exit;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

use clap::Parser;
use dialoguer::Select;
use dialoguer::console::Term;
use dialoguer::theme::ColorfulTheme;
use indicatif::{FormattedDuration, ProgressBar, ProgressStyle};
use log::{error, info, trace};

use player::Player;
use constants::{DEFAULT_DEST_PORT, DEFAULT_SRC_PORT, DEFAULT_TTL};
use crate::constants::{ERROR_CREATE_PLAYER, ERROR_INCORRECT_FILE_PATH, ERROR_INIT_PLAYER, ERROR_INIT_PLAYER_TIMEOUT, ERROR_PARSE_FILE, PLAYER_STARTUP_TIMEOUT_MS};
use crate::model::pcap::Pcap;
use crate::model::{Command, Event, PositionChange, Recording};
use crate::player::PlayerState;

#[derive(Parser, Debug)]
#[clap(name = "packet-play")]
#[clap(author, version, about,long_about = None)]
struct Cli {
    file: String,
    #[clap(parse(try_from_str))]
    #[clap(short, long, default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST),DEFAULT_DEST_PORT))]
    destination: SocketAddr,
    #[clap(short = 's', long = "source", default_value_t = DEFAULT_SRC_PORT)]
    source_port: u16,
    #[clap(short, long, default_value_t = DEFAULT_TTL)]
    ttl: u32,
    #[clap(short, long)]
    auto_play_disable: bool,
}

const SELECT_UNSUPPORTED_KEY_INPUT: usize = 99;

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let cli = Cli::parse();

    info!("Settings:");
    info!("\t Recording:\t{}", cli.file);
    info!("\t Destination:\t{}", cli.destination);
    info!("\t Source port:\t{}", cli.source_port);
    info!("\t TTL:\t\t{}", cli.ttl);
    info!("\t Auto play:\t{}", !cli.auto_play_disable);

    let file_path = std::path::Path::new(cli.file.as_str());
    if !file_path.is_file() || !file_path.exists() {
        error!("Provided path {} is not a file or does not exist.", {cli.file});
        exit(ERROR_INCORRECT_FILE_PATH);
    };

    let file = File::open(file_path).unwrap();
    let recording = Pcap::try_from(file);

    if let Ok(recording) = recording {
        let (cmd_sender, cmd_receiver) = mpsc::channel();
        let input_cmd_sender = cmd_sender.clone();
        let (event_sender, event_receiver) = mpsc::channel();
        let input_event_sender = event_sender.clone();

        let progress_bar = ProgressBar::new(recording.packets.len() as u64);

        progress_bar.set_style(ProgressStyle::default_bar()
            .template("{msg} [{wide_bar:.cyan/blue}] {pos:>7}/{len:7}")
            .progress_chars("#>-"));
        progress_bar.set_draw_rate(10);

        // TODO handle errors on creation of player
        let player_handle = match Player::builder()
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
        let input_handle = thread::spawn(move || {
            loop {
                let selection = Select::with_theme(&ColorfulTheme::default())
                    .items(&Command::as_vec())
                    .default(0)
                    .report(true)
                    .clear(true)
                    .interact_on_opt(&Term::stdout()).expect("inner").unwrap_or(SELECT_UNSUPPORTED_KEY_INPUT);

                let command = Command::from(selection);
                if let Err(_err) = input_cmd_sender.send(command) {
                    break;
                }
                if command == Command::Quit {
                    let _ = input_event_sender.send(Event::QuitCommanded);
                    break;
                }
            }
        });
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

        let mut current_state = PlayerState::Initial;
        let mut current_position = PositionChange::default();

        loop {
            let data_updated = match event_receiver.try_recv() {
                Ok(Event::QuitCommanded) => { break; }
                Ok(Event::PlayerReady) => { false }
                Ok(Event::PlayerStateChanged(state)) => {
                    current_state = state.state;
                    true
                }
                Ok(Event::PlayerPositionChanged(position)) => {
                    current_position = position;
                    progress_bar.set_position(current_position.position as u64);
                    true
                }
                Ok(Event::Error(error)) => { trace!("{error:?}"); false }
                Err(TryRecvError::Empty) => { false }
                Err(TryRecvError::Disconnected) => {
                    trace!("Event channel disconnected, Player stopped working. Exiting.");
                    break;
                }
            };

            if data_updated {
                progress_bar.set_message(format!("{} [{}]", current_state, FormattedDuration(current_position.time_position)));
            }
            else {
                progress_bar.tick();
            }
        }

        player_handle.join().expect("Player thread failed.");
        input_handle.join().expect("Input thread failed.");
    } else {
        let error = recording.unwrap_err();
        error!("Cannot play recording, because: {:?}", error);
        exit(ERROR_PARSE_FILE);
    };
}