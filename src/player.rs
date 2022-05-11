use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

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

        let mut previous_ts = duration_from_timestamp(&recording.header.magic_number, &recording.packets.first().unwrap());
        let last_ts = duration_from_timestamp(&recording.header.magic_number, &recording.packets.last().unwrap());
        let total_duration = FormattedDuration(last_ts - previous_ts);
        info!("Recording duration {}", total_duration);

        let strip_headers_index = (ETHERNET_HEADER_LENGTH+IP_HEADER_LENGTH+UDP_HEADER_LENGTH+1) as usize;

        let mut packets = recording.packets.iter().enumerate();
        let mut terminal_synced = false;

        loop {
            // receive any command and update state
            self.state = match self.cmd_rx.try_recv() {
                Ok(Command::Play) => { if self.state == PlayerState::Initial {
                        self.progress_bar.set_message("Playing");
                        self.progress_bar.reset_elapsed();
                    }
                    PlayerState::Playing
                }
                Ok(Command::Pause) => {
                    self.progress_bar.set_message("Paused");
                    PlayerState::Paused
                }
                Ok(Command::Rewind) => {
                    packets = recording.packets.iter().enumerate();
                    previous_ts = duration_from_timestamp(&recording.header.magic_number, &recording.packets.first().unwrap());
                    self.progress_bar.reset();
                    self.progress_bar.set_message("Stopped");
                    PlayerState::Initial
                }
                Ok(Command::Quit) => { PlayerState::Quit }
                Ok(Command::Unspecified) => { self.state } // no-op
                Ok(Command::SyncTerm) => {
                    self.progress_bar.set_message("Ready");
                    terminal_synced = true;
                    self.state
                } // no-op
                Err(TryRecvError::Empty) => { self.state } // no-op
                Err(TryRecvError::Disconnected) => {
                    error!("Command channel disconnected, stopping program execution.");
                    PlayerState::Quit
                }
            };

            // act on current state
            match self.state {
                PlayerState::Initial => { if terminal_synced {
                    self.progress_bar.set_message("Ready");
                } } // no-op
                PlayerState::Playing => {
                    if let Some((i, packet)) = packets.next() {
                        let current_ts = duration_from_timestamp(&recording.header.magic_number, &packet);
                        let diff = current_ts - previous_ts;
                        // TODO subtract the time it took to send the packet
                        std::thread::sleep(diff);
                        previous_ts = current_ts;
                        self.progress_bar.set_position(i as u64);
                        let _bytes_send = socket.send_to(
                            &packet.packet_data.as_slice()[strip_headers_index..],
                            self.destination)
                            .expect("Could not send packet");
                    } else {
                        self.progress_bar.finish_with_message("Finished");
                        self.state = PlayerState::Finished;
                    }
                }
                PlayerState::Paused => { if terminal_synced { self.progress_bar.tick() } } // no-op
                PlayerState::Finished => { if terminal_synced { self.progress_bar.tick() } } // no-op
                PlayerState::Quit => {
                    thread::sleep(Duration::from_millis(100));
                    break;
                }
            }
        }

        // TODO indication that we are finished
        // self.progress_bar.finish_with_message("Recording finished.");
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