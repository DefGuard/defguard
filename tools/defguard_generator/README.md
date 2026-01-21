# Defguard Object Generator

This crate contains a simple generator for creating users, devices, stats etc during development.

### Database connection

The generator uses the same environment variables (or CLI options) for DB connection setup as the core binary:

- DEFGUARD_DB_HOST
- DEFGUARD_DB_PORT
- DEFGUARD_DB_NAME
- DEFGUARD_DB_USER
- DEFGUARD_DB_PASSWORD

This means that if you have a development environment set up already it should just work.

### Usage

```bash
cargo run -p defguard_generator -- vpn-session-stats \
    --location-id 1 \
    --num-users 10 \
    --devices-per-user 2 \
    --sessions-per-device 5
```

