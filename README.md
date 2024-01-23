 <p align="center">
    <img src="docs/header.png" alt="defguard">
 </p>

defguard is a **single binary! SSO & VPN Server** based on **OpenID and WireGuard VPN** with first of it's kind **Multi-Factor Authentication for WireGuard (TOTP/Email & Pre-Shared Session Keys)** and unique secure architecture.

Read more about this in [our documentation](https://defguard.gitbook.io/defguard/#what-is-defguard).

**Implemented & production tested features:**

* [OpenID Connect provider](https://openid.net/developers/how-connect-works/) - with **unique features**:
  - Secure remote (over the internet) [user enrollment](https://defguard.gitbook.io/defguard/help/remote-user-enrollment)
  - User [onboarding after enrollment](https://defguard.gitbook.io/defguard/help/remote-user-enrollment/user-onboarding-after-enrollment)
  - LDAP (tested on [OpenLDAP](https://www.openldap.org/)) synchronization
  - [forward auth](https://defguard.gitbook.io/defguard/features/forward-auth) for reverse proxies (tested with Traefik and Caddy)
  - nice UI to manage users
  - Users **self-service** (besides typical data management, users can revoke access to granted apps, MFA, WireGuard, etc.)
  - [Multi-Factor/2FA](https://en.wikipedia.org/wiki/Multi-factor_authentication) Authentication:
   - [Time-based One-Time Password Algorithm](https://en.wikipedia.org/wiki/Time-based_one-time_password) (TOTP - e.g. Google Authenticator)
   - WebAuthn / FIDO2 - for hardware key authentication support (eg. YubiKey, FaceID, TouchID, ...)
   - Web3 - authentication with crypto software and hardware wallets using Metamask, Ledger Extension
* [WireGuard:tm:](https://www.wireguard.com/) VPN management with:
  - [Multi-Factor Authentication](https://defguard.gitbook.io/defguard/help/desktop-client/multi-factor-authentication-mfa-2fa) with TOTP/Email & Pre-Shared Session Keys
  - multiple VPN Locations (networks/sites) - with defined access (all users or only Admin group)
  - multiple [Gateways](https://github.com/DefGuard/gateway) for each VPN Location (**high availability/failover**) - supported on a cluster of routers/firewalls for Linux, FreeBSD/PFSense/OPNSense
  - **import your current WireGuard server configuration (with a wizard!)**
  - **most beautiful [Desktop Client!](https://github.com/defguard/client)** (in our opinion ;-))
  - automatic IP allocation
  - kernel (Linux, FreeBSD/OPNSense/PFSense) & userspace WireGuard support with [our Rust library](https://github.com/defguard/wireguard-rs)
  - dashboard and statistics overview of connected users/devices for admins
  - *defguard is not an official WireGuard project, and WireGuard is a registered trademark of Jason A. Donenfeld.*
* [Yubikey hardware keys](https://www.yubico.com/) provisioning for users by *one click*
* [Email/SMTP support](https://defguard.gitbook.io/defguard/help/setting-up-smtp-for-email-notifications) for notifications, remote enrollment and onboarding
* Easy support with [sending debug/support information](https://defguard.gitbook.io/defguard/help/sending-support-info)
* Webhooks & REST API
* Web3 wallet validation
* Build with [Rust](https://www.rust-lang.org/) for portability, security, and speed
* [UI Library](https://github.com/defguard/ui) - our beautiful React/TypeScript UI is a collection of React components:
  - a set of custom and beautiful components for the layout
  - Responsive Web Design (supporting mobile phones, tablets, etc..)
  - [iOS Web App](https://www.macrumors.com/how-to/use-web-apps-iphone-ipad/)
* **Checked by professional security researchers** (see [comprehensive security report](https://defguard.net/images/decap/isec-defguard.pdf))
* End2End tests

### Web management

![](https://github.com/DefGuard/docs/blob/docs/screencasts/defguard.gif?raw=true)

Better quality video can [be found here to download](https://github.com/DefGuard/docs/raw/docs/screencasts/defguard-screencast.mkv)

### Desktop Client

![defguard desktop client](https://github.com/DefGuard/docs/blob/docs/releases/0.8/Defguard-Desktop-Client.png?raw=true)

[Desktop client](https://github.com/DefGuard/client) supports:
- Secure and remote user enrollment - setting up password, automatically configuring the client for all VPN Locations/Networks
- Onboarding - displaying custom onboarding messages, with templates, links ...
- Ability to route predefined VPN traffic or all traffic (server needs to have NAT configured - in gateway example)
- Live & real-time network charts
- In development: **Multi-Factor Authentication** for VPN, live logs, dark theme, settings, and more! 

## Roadmap

[A detailed product roadmap can be found here](https://defguard.gitbook.io/defguard/features/roadmap).

## Quick start

The easiest way to run your own defguard instance is to use Docker and our [one-line install script](https://defguard.gitbook.io/defguard/features/setting-up-your-instance/one-line-install).

Just run the command below in your shell and follow the prompts:

```bash
curl --proto '=https' --tlsv1.2 -sSf -L https://raw.githubusercontent.com/DefGuard/deployment/main/docker-compose/setup.sh -O && bash setup.sh
```

To learn more about the script and available options please see the [documentation](https://defguard.gitbook.io/defguard/features/setting-up-your-instance/one-line-install).

## Manual deployment examples

* Using [Docker Compose](https://defguard.gitbook.io/defguard/features/setting-up-your-instance/docker-compose)
* Using [Kubernetes](https://defguard.gitbook.io/defguard/features/setting-up-your-instance/kubernetes)

## Why?

The story and motivation behind defguard [can be found here: https://teonite.com/blog/defguard/](https://teonite.com/blog/defguard/)

## Documentation

See the [documentation](https://defguard.gitbook.io) for more information.

## Community and Support

Find us on Matrix: [#defguard:teonite.com](https://matrix.to/#/#defguard:teonite.com)

## Contribution

Please review the [Contributing guide](https://defguard.gitbook.io/defguard/for-developers/contributing) for information on how to get started contributing to the project. You might also find our [environment setup guide](https://defguard.gitbook.io/defguard/for-developers/dev-env-setup) handy.

# Legal
WireGuard is [registered trademarks](https://www.wireguard.com/trademark-policy/) of Jason A. Donenfeld.

