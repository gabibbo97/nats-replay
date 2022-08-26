mod config;

use std::{net::SocketAddr, time::Duration};

use async_nats::jetstream::{consumer, stream::Config};
use futures_util::StreamExt;
use tokio::{net::UdpSocket, sync::mpsc};
use tracing::{debug, instrument, info};

#[instrument(skip_all)]
async fn connect_to_broker(config: &config::Config) -> anyhow::Result<async_nats::Client> {
    let client = async_nats::connect(&config.nats_address).await?;
    info!(address = config.nats_address, "Connected to NATS");
    Ok(client)
}

#[instrument(skip_all)]
async fn do_generate_dummy(tx: mpsc::Sender<Vec<u8>>) -> anyhow::Result<()> {
    let mut counter = 0;
    enum Mode {
        Slow,
        Fast,
    }
    let mut mode = Mode::Fast;
    loop {
        // Change dummy data
        counter = if counter == 99 { 0 } else { counter + 1 };
        if counter % 25 == 0 {
            mode = match mode {
                Mode::Fast => Mode::Slow,
                Mode::Slow => Mode::Fast,
            };
        }
        // Format message
        let mode_char = match mode {
            Mode::Fast => 'F',
            Mode::Slow => 'S',
        };
        let message = format!("{:02} {mode_char} {}\n", counter, chrono::Utc::now().to_rfc3339());
        // Send message
        debug!(message = message.trim(), "Generated dummy data packet");
        tx.send(message.as_bytes().to_vec()).await?;
        tokio::time::sleep(match mode {
            Mode::Fast => Duration::from_millis(100),
            Mode::Slow => Duration::from_secs(1),
        }).await;
    }
}

#[instrument(skip_all)]
async fn do_receive(bind_address: SocketAddr, buffer_size: usize, tx: mpsc::Sender<Vec<u8>>) -> anyhow::Result<()> {
    let sock = UdpSocket::bind(bind_address).await?;
    debug!(bind_address = ?sock.local_addr().unwrap(), "Bound socket");
    let mut buffer = vec![0; buffer_size];
    loop {
        let received_size = sock.recv(&mut buffer).await?;
        debug!(size = received_size, "Received datagram");
        let received = &buffer[..received_size];
        tx.send(received.to_vec()).await?;
    }
}

#[instrument(skip_all)]
async fn do_record(config: &config::Config) -> anyhow::Result<()> {
    let client = connect_to_broker(config).await?;
    let js_client = async_nats::jetstream::new(client);

    if let config::OperationMode::Record { bind_address, buffer_size, nats_buffer_size, flush_stream, skip_stream_creation, publish_dummy_data } = &config.mode {
        // Flush stream
        if *flush_stream {
            debug!(name = config.stream_name, "Requesting stream deletion");
            match js_client.delete_stream(&config.stream_name).await {
                Ok(deletion) if deletion.success => { info!(name = config.stream_name, "Deleted stream"); },
                _ => { info!(name = config.stream_name, "Stream already absent"); }
            };
        }
        // Spawn receiving task
        let (tx, mut rx) = tokio::sync::mpsc::channel(*nats_buffer_size);
        let _receive_task = tokio::task::spawn(do_receive(*bind_address, *buffer_size, tx.clone()));
        // Create stream
        if ! *skip_stream_creation {
            js_client.get_or_create_stream(Config { 
                name: config.stream_name.clone(), 
                storage: async_nats::jetstream::stream::StorageType::File,
                ..Default::default()
            }).await.map_err(|e| anyhow::anyhow!(e))?;
            info!(name = config.stream_name, "Created stream");
        }
        // Spawn dummy data task
        if *publish_dummy_data {
            info!("Starting dummy data generator");
            let _ = tokio::task::spawn(do_generate_dummy(tx.clone()));
        }
        // Publish to stream
        while let Some(message) = rx.recv().await {
            js_client.publish(config.stream_name.clone(), message.into()).await.map_err(|e| anyhow::anyhow!(e))?;
            debug!(stream_name = config.stream_name, "Published message");
        }
        anyhow::bail!("Receiver channel closed");
    } else {
        panic!("Unexpected configuration kind");
    }
}

#[instrument(skip_all)]
async fn do_replay(config: &config::Config) -> anyhow::Result<()> {
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load config
    let config_path = std::env::var("CONFIG_PATH").unwrap_or("config.json".to_string());
    let config = tokio::fs::read_to_string(&config_path).await?;
    let config = serde_json::from_str::<config::Config>(&config)?;
    debug!("Loaded config: {:?}", config);
    
    // Dispatch command
    match config.mode {
        config::OperationMode::Record { .. } => do_record(&config).await?,
        config::OperationMode::Replay { .. } => do_replay(&config).await?,
    };
    Ok(())
}
