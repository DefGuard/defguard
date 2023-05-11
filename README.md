 <p align="center">
    <img src="docs/header.png" alt="defguard">
 </p>

Defguard is an open-source security army knife (Identity, MFA, VPN, Yubikey, Web3).

Building a secure organization has always been difficult and costly. Defguard provides a beautiful, easy-to-use (business users) and deploy (admin/devops) fundament to make your organization secure.

**Features:**

* OpenID Connect provider (with OpenLDAP synchronization)
* Wireguard :tm: VPN Management
* Multi-Factor Authentication:
  - Time-based One-Time Password Algorithm (TOTP - e.g. Google Authenticator)
  - WebAuthn / FIDO2 - for hardware key authentication support
  - Web3 - authentication with crypto software and hardware wallets using Metamask, Wallet Connect, Ledger Extension
* [Yubikey harware keys](https://www.yubico.com/) provisioning
* Webhooks
* Web3 wallet validation

 <p align="center">
    <img src="docs/network-overview.png" alt="defguard">
 </p>

# Documentation

See the [documentation](https://defguard.gitbook.io) for more information.

# Community and Support

Find us on Matrix: [#defgurd:teonite.com](https://matrix.to/#/#defguard:teonite.com)

# Deployment

* Using [Docker Compose](https://defguard.gitbook.io/defguard/features/setting-up-your-instance/docker-compose)
* Using [Kubernetes](https://defguard.gitbook.io/defguard/features/setting-up-your-instance/kubernetes)

# Development environment setup

Remember to clone DefGuard repository recursively (with protos):

```
git clone --recursive git@github.com:DefGuard/defguard.git
```

## With Docker Compose

Using Docker Compose you can setup a simple stack with:

* backend
* database (PostgreSQL)
* VPN gateway
* device connected to the gateway
* ldap

This way you'll have some live stats data to work with.

To do so follow these steps:

1. Migrate database and insert test network and device:

```
docker compose run core init-dev-env
```

2. Run the application:

Without LDAP:

```
docker compose up
```

With LDAP:

```
docker compose -f docker-compose.ldap.yaml up
```

## Cargo

To run backend without Docker, you'll need:

* PostgreSQL database
* environment variables set

Run PostgreSQL with:

```
docker compose up -d db
```

You'll find environment variables in .env file. Source them however you like (we recommend https://direnv.net/).
Once that's done, you can run backend with:

```
cargo run
```

