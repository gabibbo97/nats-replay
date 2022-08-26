#!/bin/sh
exec sudo podman run --rm -it --name nats \
    -p 4222:4222 \
    -p 8222:8222 \
    --tmpdir /var/lib/nats \
    nats:2.8.4-scratch \
        --jetstream \
        --store_dir /var/lib/nats \
        "$@"
