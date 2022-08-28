# nats-replay

This little utility allows you to easily record and replay UDP data streams, leveraging the NATS message queue

## Usage

0. Have a running NATS Jetstream server running

1. Create your config file
2. Build the program with `cargo build --release`
3. Run the program with `CONFIG_PATH=<config_path> ./target/release/nats-replay`

## Configuration

The configuration file is a JSON file

### Mandatory arguments

- **nats_address**: URL of the NATS broker (example: `nats://localhost`)
- **stream_name**: Stream name on the NATS broker
- **mode**: Which mode to run the program: `info`, `record`, or `replay`

### Record mode

In this mode the system will receive and save packets to the stream.

- **bind_address**: Bind address of the UDP socket
- **buffer_size**: Size of the UDP socket buffer (default: 16384 bytes)
- **nats_buffer_size**: Size of the NATS send buffer (default: 16384 messages)
- **flush_stream**: Whenever the stream has to be reset on startup
- **skip_stream_creation**: Whenever the stream is already existing
- **publish_dummy_data**: Periodically publish test data to the stream

### Replay mode

In this mode the system will replay packets from the stream.

- **deliver_policy**: specifies the starting point for the replay
  - `all`: All messages in the stream
  - `last`: Last message in the stream
  - `new`: Deliver only new messages
  - `by_start_sequence: { opt_start_seq: <n> }`: Starting from sequence number `n`
  - `by_start_time: { opt_start_time: "<t>" }`: Starting from time `t` (RFC3339)
- **replay_policy**: specifies how messages will be replayed
  - `instant`: Sends all messages in the stream to the consumer as quickly as possible
  - `original`: Sends messages to the stream respecting the rate of arrival
- **bind_address**: where to bind the sending socket (default: `0.0.0.0:0`)
- **send_address**: where to send the received packets
- **sock_broadcast**: sets `SO_BROADCAST` on the socket

### Info mode

In this mode the system will show data for a given stream
