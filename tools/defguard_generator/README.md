# Defguard Object Generator

This crate contains a simple generator for creating users, devices, stats etc during development.

### Usage

```bash
cargo run -p defguard_generator vpn-sessions \
    --network-id 1 \
    --users 10 \
    --devices-per-user 2 \
    --sessions-per-device 5
```

