use std::net::{IpAddr, SocketAddr, Ipv4Addr};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub nats_address: String,
    pub stream_name: String,
    #[serde(flatten)]
    pub mode: OperationMode,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "mode")]
pub enum OperationMode {
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "record")]
    Record {
        /// Bind address of the UDP socket
        bind_address: SocketAddr,
        /// Size of the UDP socket buffer
        #[serde(default = "OperationMode::default_buffer_size")]
        buffer_size: usize,
        /// Size of the NATS send buffer
        #[serde(default = "OperationMode::default_nats_buffer_size")]
        nats_buffer_size: usize,
        /// Whenever the stream has to be deleted on start
        #[serde(default)]
        flush_stream: bool,
        /// Whenever to skip the initial stream creation
        #[serde(default)]
        skip_stream_creation: bool,
        /// Publish dummy data for testing purposes
        #[serde(default)]
        publish_dummy_data: bool
    },
    #[serde(rename = "replay")]
    Replay {
        /// NATS Deliver policy
        #[serde(flatten)]
        deliver_policy: async_nats::jetstream::consumer::DeliverPolicy,
        /// NATS Replay policy
        replay_policy: async_nats::jetstream::consumer::ReplayPolicy,
        /// UDP socket bind address
        #[serde(default = "OperationMode::default_replay_bind_address")]
        bind_address: SocketAddr,
        /// UDP destination address
        dest_address: SocketAddr,
        /// Whenever the socket is broadcast
        #[serde(default)]
        sock_broadcast: bool,
    }
}
impl OperationMode {
    fn default_buffer_size() -> usize {
        16384
    }
    fn default_nats_buffer_size() -> usize {
        16384
    }
    fn default_replay_bind_address() -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)
    }
}