use std::fmt::{Display, Formatter};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use indicatif::{FormattedDuration, ProgressBar};
use log::{error, info, trace};

use crate::constants::{DEFAULT_DEST_PORT, DEFAULT_SRC_PORT, DEFAULT_TTL};
use crate::model::{Command, ETHERNET_HEADER_LENGTH, IP_HEADER_LENGTH, Recording, UDP_HEADER_LENGTH};
use crate::model::pcap::{PcapMagicNumber, PcapPacketRecord};

pub struct Player {
    recording: Recording,
    destination: SocketAddr,
    source_port: u16,
    ttl: u32,
    cmd_rx: Receiver<Command>,
    progress_bar: ProgressBar,
    state: PlayerState,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum PlayerState {
    Initial,
    Playing,
    Paused,
    Finished,
    Quit,
}

impl Display for PlayerState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerState::Initial => { write!(f, "Ready") }
            PlayerState::Playing => { write!(f, "Playing") }
            PlayerState::Paused => { write!(f, "Paused") }
            PlayerState::Finished => { write!(f, "Finished") }
            PlayerState::Quit => { write!(f, "") }
        }
    }
}

impl Player {
    pub fn new(recording: Recording, cmd_rx: Receiver<Command>, progress_bar: ProgressBar) -> Self {
        Self {
            recording,
            destination: SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST),DEFAULT_DEST_PORT),
            source_port: DEFAULT_SRC_PORT,
            ttl: DEFAULT_TTL,
            cmd_rx,
            progress_bar,
            state: PlayerState::Initial,
        }
    }

    pub fn file(self, recording: Recording) -> Self {
        Self {
            recording,
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

    pub fn play(&mut self) {
        let socket = UdpSocket::bind(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), self.source_port))
            .expect(format!("Failed to bind socket to port {:?}", self.source_port).as_str());
        socket.set_broadcast(true).expect("Failed to set socket SO_BROADCAST option.");
        socket.set_ttl(self.ttl).expect("Failed to set socket TTL value");

        let recording = if let Recording::PCAP(pcap) = &self.recording {
            pcap
        } else {
            error!("Files other than .pcap are currently not supported.");
            return;
        };
        trace!("{:?}", recording.header);

        let first_ts = duration_from_timestamp(&recording.header.magic_number, &recording.packets.first().unwrap());
        let last_ts = duration_from_timestamp(&recording.header.magic_number, &recording.packets.last().unwrap());
        let total_duration = FormattedDuration(last_ts - first_ts);
        info!("Recording duration {}", total_duration);

        const STRIP_HEADERS_INDEX: usize = (ETHERNET_HEADER_LENGTH+IP_HEADER_LENGTH+UDP_HEADER_LENGTH+1) as usize;

        let mut packets = recording.packets.iter().enumerate();
        let mut terminal_synced = false;
        let mut previous_ts = first_ts.clone();
        let mut playback_elapsed = previous_ts - first_ts;
        let mut previous_state = self.state.clone();

        let mut loop_time_start : Option<Instant> = None;

        loop {
            // receive any command and update state
            if let Some(new_state) = match self.cmd_rx.try_recv() {
                Ok(Command::Play) => { if self.state == PlayerState::Initial {
                    }
                    Some(PlayerState::Playing)
                }
                Ok(Command::Pause) => {
                    Some(PlayerState::Paused)
                }
                Ok(Command::Rewind) => {
                    packets = recording.packets.iter().enumerate();
                    previous_ts = duration_from_timestamp(&recording.header.magic_number, &recording.packets.first().unwrap());
                    playback_elapsed = Duration::new(0,0);
                    self.progress_bar.reset();
                    Some(PlayerState::Initial)
                }
                Ok(Command::Quit) => { Some(PlayerState::Quit) }
                Ok(Command::Unspecified) => { None } // no-op
                Ok(Command::SyncTerm) => {
                    terminal_synced = true;
                    None
                } // no-op
                Err(TryRecvError::Empty) => { None } // no-op
                Err(TryRecvError::Disconnected) => {
                    error!("Command channel disconnected, stopping program execution.");
                    Some(PlayerState::Quit)
                }
            } {
                self.state = new_state;
            };

            // act on current state
            match self.state {
                PlayerState::Initial => { if terminal_synced {
                    self.progress_bar.set_message(format!("{}", self.state));
                } } // no-op
                PlayerState::Playing => {
                    if let Some((i, packet)) = packets.next() {
                        let current_ts = duration_from_timestamp(&recording.header.magic_number, &packet);
                        let ts_duration = current_ts.saturating_sub(previous_ts);

                        let loop_duration = if let Some(start) = loop_time_start {
                            start.elapsed()
                        } else { Duration::new(0, 0) };

                        std::thread::sleep(ts_duration.saturating_sub(loop_duration));

                        loop_time_start = Some(Instant::now());

                        previous_ts = current_ts;
                        playback_elapsed = current_ts - first_ts;

                        self.progress_bar.set_position((i+1) as u64);

                        let _bytes_send = socket.send_to(
                            &packet.packet_data.as_slice()[STRIP_HEADERS_INDEX..],
                            self.destination)
                            .expect("Could not send packet");
                    } else {
                        self.progress_bar.finish();
                        self.state = PlayerState::Finished;
                    }
                }
                PlayerState::Paused => { if terminal_synced { self.progress_bar.tick() } } // no-op
                PlayerState::Finished => { if terminal_synced { self.progress_bar.tick() } } // no-op
                PlayerState::Quit => {
                    break;
                }
            }
            if previous_state != self.state || self.state == PlayerState::Playing {
                self.progress_bar.set_message(format!("{} [{}]", self.state, FormattedDuration(playback_elapsed)));
            }
            previous_state = self.state.clone();
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

// TODO progress bar elapsed time progresses even while paused; replace with time calculation based on the timestamps