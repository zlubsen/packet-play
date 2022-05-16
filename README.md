# Packet-play

A CLI tool to replay .pcap and .pcapng files on networks.
- Support for resending UDP messages.
- Support for altering the destination address and port of the packets.
- Support for setting the source port of the packets.
- Support for setting the ttl of the packets.
- Assumes the packets have been recorded using Ethernet/IP/UDP.
- VCR-like controls: play, pause, rewind, quit.

Usage notes:
- Use `--help` for a list of arguments
- By default, the player will start to repay the provided file immediately. The `-a` flag overrides this behavior.

Minimal usage:
```sh
./packet-play[.exe] path/to/your/file.pcap
```

Future work:
- Support .pcapng files