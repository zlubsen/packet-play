mod model;
mod player;
mod constants;

use std::{env, thread};
use std::fs::File;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::exit;
use std::sync::mpsc;
use std::time::Duration;

use clap::Parser;
use dialoguer::Select;
use dialoguer::console::Term;
use dialoguer::theme::ColorfulTheme;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use log::{error, info, trace};

use player::Player;
use constants::{DEFAULT_DEST_PORT, DEFAULT_SRC_PORT, DEFAULT_TTL};
use crate::model::pcap::Pcap;
use crate::model::{Command, Recording};

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

    info!("Using file: {}", cli.file);
    info!("Replaying to destination: {:?}", cli.destination);

    info!("Auto play: {}", cli.auto_play_disable);

    let file_path = std::path::Path::new(cli.file.as_str());
    if !file_path.is_file() || !file_path.exists() {
        error!("Provided path {} is not a file or does not exist.", {cli.file});
        exit(1);
    };

    let file = File::open(file_path).unwrap();
    let recording = Pcap::try_from(file);

    if let Ok(recording) = recording {
        let (sender, receiver) = mpsc::channel();

        let bar = ProgressBar::new(recording.packets.len() as u64);

        bar.set_style(ProgressStyle::default_bar()
            // .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos:>7}/{len:7}")
            .template("{msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos:>7}/{len:7}")
            .progress_chars("#>-"));

        let player_bar = bar.clone();

        let player_handle = thread::spawn(move || {
            let mut player = Player::new(Recording::PCAP(recording), receiver, player_bar).destination(cli.destination).source_port(cli.source_port).ttl(cli.ttl);
            player.play();
        });
        thread::sleep(Duration::from_millis(500)); // Give the player time to setup and output messages to terminal.

        if !cli.auto_play_disable {
            sender.send(Command::Play).expect("Auto play failed.");
        }

        let mut initialised = false;

        loop {
            let selection = Select::with_theme(&ColorfulTheme::default())
                .items(&Command::as_vec())
                .default(0)
                .report(true)
                .clear(true)
                .interact_on_opt(&Term::stdout()).expect("inner").unwrap_or(SELECT_UNSUPPORTED_KEY_INPUT);

            if !initialised {
                sender.send(Command::SyncTerm);
            }

            let command = Command::from(selection);
            if let Err(err) = sender.send(command) {
                break;
            }
            if command == Command::Quit {
                break;
            }
        }

        player_handle.join().expect("Player thread failed.");
    } else {
        let error = recording.unwrap_err();
        error!("Cannot play recording, because: {:?}", error);
        exit(1);
    };
}