mod model;
mod player;
mod constants;
mod cli;
mod gui;

use std::env;
use std::fs::File;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::exit;

use clap::Parser;
use log::error;

use constants::{DEFAULT_DEST_PORT, DEFAULT_SRC_PORT, DEFAULT_TTL};
use crate::constants::{ERROR_INCORRECT_FILE_PATH, ERROR_PARSE_FILE};
use crate::model::pcap::Pcap;
use crate::model::Mode;

#[derive(Parser, Debug)]
#[clap(name = "packet-play")]
#[clap(author, version, about,long_about = None)]
pub(crate) struct Cli {
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
    #[clap(parse(try_from_str))]
    #[clap(short, long, default_value_t = String::from("cli"))]
    mode: String,
}

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let cli = Cli::parse();
    let mode : Mode = cli.mode.parse::<Mode>().unwrap();

    let file_path = std::path::Path::new(cli.file.as_str());
    if !file_path.is_file() || !file_path.exists() {
        error!("Provided path {} is not a file or does not exist.", {cli.file});
        exit(ERROR_INCORRECT_FILE_PATH);
    };

    let file = File::open(file_path).unwrap();
    let recording = Pcap::try_from(file);

    if let Ok(recording) = recording {
        match mode {
            Mode::Cli => { cli::run_cli(cli, recording) }
            Mode::Gui => { gui::run_gui(cli, recording) }
        }
    } else {
        let error = recording.unwrap_err();
        error!("Cannot play recording, because: {:?}", error);
        exit(ERROR_PARSE_FILE);
    };
}