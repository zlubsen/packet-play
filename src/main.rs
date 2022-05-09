mod model;

use std::env;
use std::fs::File;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::process::exit;
use std::time::{Duration};
use clap::Parser;
use dialoguer::Confirm;
use dialoguer::console::Term;
use indicatif::{FormattedDuration, ProgressBar, ProgressStyle};
use log::{info, error, trace};
use crate::model::{ETHERNET_HEADER_LENGTH, IP_HEADER_LENGTH, UDP_HEADER_LENGTH};
use crate::model::pcap::{PcapMagicNumber, PcapPacketRecord};

const DEFAULT_DEST_PORT : u16 = 3000;
const DEFAULT_SRC_PORT : u16 = 3000;
const DEFAULT_TTL : u32 = 1;

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
}

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    let cli = Cli::parse();

    info!("Using file: {}", cli.file);
    info!("Replaying to destination: {:?}", cli.destination);

    let file_path = std::path::Path::new(cli.file.as_str());
    if !file_path.is_file() || !file_path.exists() {
        error!("Provided path {} is not a file or does not exist.", {cli.file});
        exit(1);
    };

    let player = Player::new(cli.file).destination(cli.destination).source_port(cli.source_port).ttl(cli.ttl);
    player.play();
}

struct Player {
    file_path: String,
    destination: SocketAddr,
    source_port: u16,
    ttl: u32,
}

impl Player {
    pub fn new(file_path: String) -> Self {
        Self {
            file_path,
            destination: SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST),DEFAULT_DEST_PORT),
            source_port: DEFAULT_SRC_PORT,
            ttl: DEFAULT_TTL,
        }
    }

    pub fn file(self, file_path: String) -> Self {
        Self {
            file_path,
            ..self
        }
    }

    pub fn destination(self, destination: SocketAddr) -> Self {
        Self {
            destination,
            ..self
        }
    }

    pub fn source_port(self, source_port: u16) -> Self {
        Self {
            source_port,
            ..self
        }
    }

    pub fn ttl(self, ttl: u32) -> Self {
        Self {
            ttl,
            ..self
        }
    }

    pub fn play(&self) {
        let socket = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), self.source_port)).expect(format!("Failed to bind socket to port {:?}", self.source_port).as_str());
        socket.set_broadcast(true).expect("Failed to set socket SO_BROADCAST option.");
        socket.set_ttl(self.ttl).expect("Failed to set socket TTL value");

        let file = File::open(self.file_path.clone()).unwrap();
        let recording = model::pcap::Pcap::try_from(file);

        if let Ok(recording) = recording {
            trace!("{:?}", recording.header);

            let bar = ProgressBar::new(recording.packets.len() as u64);
            bar.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos:>7}/{len:7}")
                .progress_chars("#>-"));

            let mut previous_ts = duration_from_timestamp(&recording.header.magic_number, &recording.packets.first().unwrap());
            let last_ts = duration_from_timestamp(&recording.header.magic_number, &recording.packets.last().unwrap());
            let total_duration = FormattedDuration(last_ts - previous_ts);
            info!("Recording duration {}", total_duration);

            let strip_headers_index = (ETHERNET_HEADER_LENGTH+IP_HEADER_LENGTH+UDP_HEADER_LENGTH+1) as usize;

            match Confirm::new().with_prompt("Play recording?").interact_on_opt(&Term::stdout()).unwrap() {
                None | Some(false) => { info!("Ok, bye."); exit(0); }
                Some(true) => { }
            }

            for (i, packet) in recording.packets.iter().enumerate() {
                let current_ts = duration_from_timestamp(&recording.header.magic_number, &packet);
                let diff = current_ts - previous_ts;
                std::thread::sleep(diff);
                previous_ts = current_ts;
                bar.set_position(i as u64);
                let _bytes_send = socket.send_to(
                    &packet.packet_data.as_slice()[strip_headers_index..],
                    self.destination)
                    .expect("Could not send packet");
            }

            bar.finish_with_message("Recording finished.");
        } else {
            let error = recording.unwrap_err();
            error!("Cannot play recording, because: {:?}", error);
        }
    }
}

fn duration_from_timestamp(mode: &PcapMagicNumber, packet: &PcapPacketRecord) -> Duration {
    let (fraction, overflow) = match mode {
        PcapMagicNumber::LeMicros => {
            packet.ts_secs_fraction.overflowing_mul(1_000)
        }
        PcapMagicNumber::BeNanos => { (packet.ts_secs_fraction, false) }
    };
    let seconds = if overflow {
        packet.ts_secs + 1
    } else {
        packet.ts_secs
    } as u64;
    Duration::new(seconds, fraction)
}

// TODO controls like pause, stop, rewind (put recording in separate thread?), Player state machine
// TODO dialog to replay at finish of replaying