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
    --num-users 10 \
    --devices-per-user 2 \
    --sessions-per-device 5

# optional: limit generation to one VPN location --location-id <ID>
```

### Devices

Creates users, user devices and network devices without VPN sessions or location assignments,
for example to fill the devices table with data:

```bash
cargo run -p defguard_generator -- devices \
    --num-users 200 \
    --devices-per-user 10 \
    --network-devices 50
```

Counts are targets, not increments: existing users and devices are reused, so running the same
command again adds nothing. Network devices are assigned to the first admin as their creator and
have no location, so the Network Devices page skips them.

### Session generation logic

For each device the generator always starts with creating an active (not disconnected) session.
If there are more sessions per device to be generated it goes backwards in time and creates
additional disconnected sessions.
Session duration and gaps between sessions are randomized but there is no logic to verify if
sessions are overlapping so by default the generator runs a `TRUNCATE` query at the start.
To disable this behavior (for example when running it multiple times for separate locations)
use the `--no-truncate` CLI flag.
