# Defguard Object Generator

This crate contains a simple generator for creating users, devices, stats etc during development.

### Usage

```bash
cargo run -p defguard_generator -- vpn-session-stats \
    --location-id 1 \
    --num-users 10 \
    --devices-per-user 2 \
    --sessions-per-device 5
```

