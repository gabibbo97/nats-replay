use std::{time::Duration, net::SocketAddr};

use tokio::{sync::mpsc, net::UdpSocket};
use tracing::{instrument, debug, info};

use crate::{connect_to_broker, config};

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
pub async fn do_record(config: &config::Config) -> anyhow::Result<()> {
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
            js_client.get_or_create_stream(async_nats::jetstream::stream::Config { 
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