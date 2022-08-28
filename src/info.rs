use tracing::{instrument, error, info};

use crate::{config, connect_to_broker};

#[instrument(skip_all)]
pub async fn do_get_stream_info(config: &config::Config) -> anyhow::Result<()> {
    let client = connect_to_broker(config).await?;
    let js_client = async_nats::jetstream::new(client);

    if let Ok(stream) = js_client.get_stream(&config.stream_name).await {
        let stream_info = stream.info;
        
        let stream_state = stream_info.state;

        let created = stream_info.created.to_string();

        let start_i = stream_state.first_sequence;
        let end_i = stream_state.last_sequence;

        let start_t = stream_state.first_timestamp.to_string();
        let end_t = stream_state.last_timestamp.to_string();

        let size_bytes = stream_state.bytes;
        let num_messages = stream_state.messages;

        info!(created, start_i, end_i, start_t, end_t, size_bytes, num_messages, "Stream info");
    } else {
        error!(name = config.stream_name, "Requested stream not found");
    }

    Ok(())
}