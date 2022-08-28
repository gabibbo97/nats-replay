use async_nats::jetstream::consumer;
use futures_util::StreamExt;
use tokio::net::UdpSocket;
use tracing::{instrument, debug};

use crate::{config, connect_to_broker};

#[instrument(skip_all)]
pub async fn do_replay(config: &config::Config) -> anyhow::Result<()> {
    let client = connect_to_broker(config).await?;
    let js_client = async_nats::jetstream::new(client);

    if let config::OperationMode::Replay { deliver_policy, replay_policy, bind_address, dest_address, sock_broadcast  } = &config.mode {
        let stream = js_client.get_stream(&config.stream_name).await.map_err(|e| anyhow::anyhow!(e))?;
        let subscription = stream.create_consumer(async_nats::jetstream::consumer::push::Config {
            deliver_policy: *deliver_policy,
            deliver_subject: format!("stream-{}-delivery-creation-{}", config.stream_name, chrono::Utc::now().to_rfc3339()),
            ack_policy: consumer::AckPolicy::Explicit,
            replay_policy: *replay_policy,
            ..Default::default()
         }).await.map_err(|e| anyhow::anyhow!(e))?;
        // Prepare send socket
        let sock = UdpSocket::bind(*bind_address).await?;
        if *sock_broadcast {
            sock.set_broadcast(true)?;
        }
        // Replay stream
        let mut messages = subscription.messages().await.map_err(|e| anyhow::anyhow!(e))?;
        while let Some(Ok(message)) = messages.next().await {
            debug!(size = message.payload.len(), "Received message");
            sock.send_to(&message.payload, dest_address).await?;
            message.ack().await.map_err(|e| anyhow::anyhow!(e))?;
        }
        anyhow::bail!("Subscription closed");
    } else {
        panic!("Unexpected configuration kind");
    }
}