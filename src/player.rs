use std::fmt::{Display, Formatter};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use log::trace;

use crate::model::{Command, Error, ETHERNET_HEADER_LENGTH, Event, IP_HEADER_LENGTH, Recording, UDP_HEADER_LENGTH};
use crate::model::pcap::{PcapMagicNumber, PcapPacketRecord};

pub struct Player {
    recording: Recording,
    destination: SocketAddr,
    source_port: u16,
    ttl: u32,
    state: PlayerState,
    cmd_rx: Receiver<Command>,
    event_tx: Sender<Event>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum PlayerState {
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
    pub fn run(&mut self) {
        const STRIP_HEADERS_INDEX: usize = (ETHERNET_HEADER_LENGTH+IP_HEADER_LENGTH+UDP_HEADER_LENGTH+1) as usize;

        let socket = UdpSocket::bind(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), self.source_port))
            .expect(format!("Failed to bind socket to port {:?}", self.source_port).as_str());
        socket.set_broadcast(true).expect("Failed to set socket SO_BROADCAST option.");
        socket.set_ttl(self.ttl).expect("Failed to set socket TTL value");

        let recording = if let Recording::PCAP(pcap) = &self.recording {
            pcap
        } else {
            let _ = self.event_tx.send(
                Event::error(
                    Error::FileTypeNotSupported("Files other than .pcap are currently not supported.".to_string()
            )));
            return;
        };
        trace!("{:?}", recording.header);

        let first_ts = duration_from_timestamp(&recording.header.magic_number, &recording.packets.first().unwrap());
        let last_ts = duration_from_timestamp(&recording.header.magic_number, &recording.packets.last().unwrap());
        let total_duration = last_ts - first_ts;

        let mut packets = recording.packets.iter().enumerate();
        // let mut terminal_synced = false;
        let mut previous_ts = first_ts.clone();
        let mut playback_elapsed = previous_ts - first_ts;
        // let mut previous_state = self.state.clone();

        let _ = self.event_tx.send(Event::PlayerReady);
        let _ = self.event_tx.send(Event::state_event(PlayerState::Initial));
        let _ = self.event_tx.send(Event::position_event(0,recording.packets.len(),playback_elapsed, total_duration));

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
                    let _ = self.event_tx.send(
                        Event::position_event(
                            0 ,packets.len(),
                            playback_elapsed.clone(), total_duration.clone()));
                    // self.progress_bar.reset();
                    Some(PlayerState::Initial)
                }
                Ok(Command::Quit) => { Some(PlayerState::Quit) }
                Ok(Command::Unspecified) => { None } // no-op
                // Ok(Command::SyncTerm) => {
                //     // terminal_synced = true;
                //     None
                // } // no-op
                Err(TryRecvError::Empty) => { None } // no-op
                Err(TryRecvError::Disconnected) => {
                    let _ = self.event_tx.send(Event::error(Error::CommandChannelError)).unwrap();
                    Some(PlayerState::Quit)
                }
            } {
                let _ = self.event_tx.send(Event::state_event(new_state));
                self.state = new_state;
            };

            // TODO review need for terminal_synced, if we send the state before entering the main loop
            // act on current state
            match self.state {
                PlayerState::Initial => {
                    // if terminal_synced {
                    //     let _ = self.event_tx.send(Event::PlayerStateChanged(StateChange { state: PlayerState::Initial }));
                    // self.progress_bar.set_message(format!("{}", self.state));
                } //} // no-op
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

                        // self.progress_bar.set_position((i+1) as u64);
                        let _ = self.event_tx.send(Event::position_event(
                            i,
                            recording.packets.len(),
                            playback_elapsed.clone(),
                            total_duration.clone()
                        ));

                        let _bytes_send = socket.send_to(
                            &packet.packet_data.as_slice()[STRIP_HEADERS_INDEX..],
                            self.destination)
                            .expect("Could not send packet");
                    } else {
                        // self.progress_bar.finish();
                        let _ = self.event_tx.send(Event::state_event(PlayerState::Finished));
                        self.state = PlayerState::Finished;
                    }
                }
                PlayerState::Paused => { /*if terminal_synced { self.progress_bar.tick() }*/ } // no-op
                PlayerState::Finished => { /*if terminal_synced { self.progress_bar.tick() }*/ } // no-op
                PlayerState::Quit => {
                    let _ = self.event_tx.send(Event::QuitCommanded);
                    break;
                }
            }
            // if previous_state != self.state || self.state == PlayerState::Playing {
            //     self.progress_bar.set_message(format!("{} [{}]", self.state, FormattedDuration(playback_elapsed)));
            // }
            // previous_state = self.state.clone();
        }
    }

    pub(crate) fn builder() -> PlayerBuilder {
        PlayerBuilder {
            recording: None,
            destination: None,
            source_port: None,
            ttl: None,
            cmd_rx: None,
            event_tx: None,
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

pub(crate) struct PlayerBuilder {
    recording: Option<Recording>,
    destination: Option<SocketAddr>,
    source_port: Option<u16>,
    ttl: Option<u32>,
    cmd_rx: Option<Receiver<Command>>,
    event_tx: Option<Sender<Event>>,
}

impl PlayerBuilder {
    pub(crate) fn recording(mut self, recording : Recording) -> Self {
        self.recording = Some(recording);
        self
    }

    pub(crate) fn destination(self, destination: SocketAddr) -> Self {
        Self {
            destination : Some(destination),
            ..self
        }
    }

    pub(crate) fn source_port(self, source_port: u16) -> Self {
        Self {
            source_port : Some(source_port),
            ..self
        }
    }

    pub(crate) fn ttl(self, ttl: u32) -> Self {
        Self {
            ttl : Some(ttl),
            ..self
        }
    }

    pub(crate) fn cmd_rx(self, cmd_rx: Receiver<Command>) -> Self {
        Self {
            cmd_rx : Some(cmd_rx),
            ..self
        }
    }

    pub(crate) fn event_tx(self, event_tx: Sender<Event>) -> Self {
        Self {
            event_tx : Some(event_tx),
            ..self
        }
    }

    pub(crate) fn build(self) -> Result<JoinHandle<()>, Error> {
        if self.recording.is_none() ||
            self.destination.is_none() ||
            self.source_port.is_none() ||
            self.ttl.is_none() ||
            self.cmd_rx.is_none() ||
            self.event_tx.is_none() {
            return Err(Error::PlayerInitError)
        }
        let mut player = Player {
            recording: self.recording.unwrap(),
            destination: self.destination.unwrap(),
            source_port: self.source_port.unwrap(),
            ttl: self.ttl.unwrap(),
            state: PlayerState::Initial,
            cmd_rx: self.cmd_rx.unwrap(),
            event_tx: self.event_tx.unwrap(),
        };
        Ok(thread::spawn(move || {
            player.run();
        }))
    }
}

// TODO progress bar elapsed time progresses even while paused; replace with time calculation based on the timestamps