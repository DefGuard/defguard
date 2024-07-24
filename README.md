<div align="center">
 <p align="center">
    <img src="docs/header.png" alt="defguard">
    <h3>The only open-source solution with real WireGuard MFA/2FA & integrated OpenID Connect SSO</h3>
    <img alt="GitHub commits since latest release" src="https://img.shields.io/github/commits-since/defguard/defguard/latest/dev?style=for-the-badge&label=COMMITS%20SINCE%20LATEST%20RELEASE">
 </p>

[Website](https://defguard.net) | [Getting Started](https://defguard.gitbook.io/defguard/#what-is-defguard) | [Features](https://github.com/defguard/defguard#features) | [Roadmap](https://github.com/orgs/defguard/projects/5) | [Support ❤](https://github.com/defguard/defguard#support-)

</div>

-  SSO, VPN, and hardware security key management combined, which provides:
    - significant cost saving, simplifying deployment and maintenance
    - enabling features unavailable to VPN platforms relying upon 3rd party SSO integration
- Real [WireGuard® MFA](https://defguard.gitbook.io/defguard/admin-and-features/wireguard/multi-factor-authentication-mfa-2fa/architecture) (not 2FA to "access application" like most solutions)
- Secure and robust architecture, featuring components and micro-services seamlessly deployable in diverse network setups (eg. utilizing  network segments like Demilitarized Zones, Intranet with no external access, etc), ensuring a secure environment.
- Enterprise ready (multiple Locations/Gateways/Kubernetes deployment, etc..)
- Build on WireGuard® protocol which is faster than IPSec, and significantly faster than OpenVPN
- Build with Rust for speed and security

See below [full list of features](https://github.com/defguard/defguard#features)

### Control plane management

![](https://github.com/DefGuard/docs/blob/docs/screencasts/defguard.gif?raw=true)

Better quality video can [be found here to download](https://github.com/DefGuard/docs/raw/docs/screencasts/defguard-screencast.mkv)

### Desktop Client with 2FA / MFA (Multi-Factor Authentication)

![defguard WireGuard MFA](https://github.com/DefGuard/docs/blob/docs/releases/0.9/mfa.png?raw=true)

[Desktop client](https://github.com/DefGuard/client):
- **2FA / Multi-Factor Authentication** with TOTP or email based tokens & WireGuard PSK
- Secure and remote user enrollment - setting up password, automatically configuring the client for all VPN Locations/Networks
- Onboarding - displaying custom onboarding messages, with templates, links ...
- Ability to route predefined VPN traffic or all traffic (server needs to have NAT configured - in gateway example)
- Live & real-time network charts
- live VPN logs
- light/dark theme
- 
## Quick start

The easiest way to run your own defguard instance is to use Docker and our [one-line install script](https://defguard.gitbook.io/defguard/features/setting-up-your-instance/one-line-install).

Just run the command below in your shell and follow the prompts:

```bash
curl --proto '=https' --tlsv1.2 -sSf -L https://raw.githubusercontent.com/DefGuard/deployment/main/docker-compose/setup.sh -O && bash setup.sh
```

To learn more about the script and available options please see the [documentation](https://defguard.gitbook.io/defguard/features/setting-up-your-instance/one-line-install).

### Setup a VPN server under 5min!?

Just follow [this tutorial](http://bit.ly/defguard-setup)

## Manual deployment examples

* Using [Docker Compose](https://defguard.gitbook.io/defguard/features/setting-up-your-instance/docker-compose)
* Using [Kubernetes](https://defguard.gitbook.io/defguard/features/setting-up-your-instance/kubernetes)

## Roadmap & Development backlog

[A detailed product roadmap and development status can be found here](https://github.com/orgs/DefGuard/projects/5/views/1)

### ⛑️ Want to help? ⛑️ 

Here is a [dedicated view for **good first bugs**](https://github.com/orgs/DefGuard/projects/5/views/5)

## Why?

The story and motivation behind defguard [can be found here: https://teonite.com/blog/defguard/](https://teonite.com/blog/defguard/)

## Features

* [WireGuard®](https://www.wireguard.com/) VPN server with:
  - Real and unique [Multi-Factor Authentication](https://defguard.gitbook.io/defguard/help/desktop-client/multi-factor-authentication-mfa-2fa) with TOTP/Email & Pre-Shared Session Keys
  - multiple VPN Locations (networks/sites) - with defined access (all users or only Admin group)
  - multiple [Gateways](https://github.com/DefGuard/gateway) for each VPN Location (**high availability/failover**) - supported on a cluster of routers/firewalls for Linux, FreeBSD/PFSense/OPNSense
  - **import your current WireGuard® server configuration (with a wizard!)**
  - **most beautiful [Desktop Client!](https://github.com/defguard/client)** (in our opinion ;-))
  - automatic IP allocation
  - kernel (Linux, FreeBSD/OPNSense/PFSense) & userspace WireGuard® support with [our Rust library](https://github.com/defguard/wireguard-rs)
  - dashboard and statistics overview of connected users/devices for admins
  - *defguard is not an official WireGuard® project, and WireGuard is a registered trademark of Jason A. Donenfeld.*
* Integrated SSO: [OpenID Connect provider](https://openid.net/developers/how-connect-works/) - with **unique features**:
  - Secure remote (over the internet) [user enrollment](https://defguard.gitbook.io/defguard/help/remote-user-enrollment)
  - User [onboarding after enrollment](https://defguard.gitbook.io/defguard/help/remote-user-enrollment/user-onboarding-after-enrollment)
  - LDAP (tested on [OpenLDAP](https://www.openldap.org/)) synchronization
  - [forward auth](https://defguard.gitbook.io/defguard/features/forward-auth) for reverse proxies (tested with Traefik and Caddy)
  - nice UI to manage users
  - Users **self-service** (besides typical data management, users can revoke access to granted apps, MFA, WireGuard®, etc.)
  - [Multi-Factor/2FA](https://en.wikipedia.org/wiki/Multi-factor_authentication) Authentication:
   - [Time-based One-Time Password Algorithm](https://en.wikipedia.org/wiki/Time-based_one-time_password) (TOTP - e.g. Google Authenticator)
   - WebAuthn / FIDO2 - for hardware key authentication support (eg. YubiKey, FaceID, TouchID, ...)
   - Email based TOTP
* Extenal SSO: [External OpenID Providers support](https://defguard.gitbook.io/defguard/admin-and-features/external-openid-providers) - *in testing, [watch this issue](https://github.com/DefGuard/defguard/issues/602)* - Google, Microsoft or custom 
* SSH & GPG public key management in user profile - with [SSH keys authentication for servers](https://defguard.gitbook.io/defguard/admin-and-features/ssh-authentication)
* [Yubikey hardware keys](https://www.yubico.com/) provisioning for users by *one click*
* [Email/SMTP support](https://defguard.gitbook.io/defguard/help/setting-up-smtp-for-email-notifications) for notifications, remote enrollment and onboarding
* Easy support with [sending debug/support information](https://defguard.gitbook.io/defguard/help/sending-support-info)
* Webhooks & REST API
* Build with [Rust](https://www.rust-lang.org/) for portability, security, and speed
* [UI Library](https://github.com/defguard/ui) - our beautiful React/TypeScript UI is a collection of React components:
  - a set of custom and beautiful components for the layout
  - Responsive Web Design (supporting mobile phones, tablets, etc..)
  - [iOS Web App](https://www.macrumors.com/how-to/use-web-apps-iphone-ipad/)
* **Checked by professional security researchers** (see [comprehensive security report](https://defguard.net/images/decap/isec-defguard.pdf))
* End2End tests

## Documentation

See the [documentation](https://defguard.gitbook.io) for more information.

## Community and Support

Find us on Matrix: [#defguard:teonite.com](https://matrix.to/#/#defguard:teonite.com)

## Contribution

Please review the [Contributing guide](https://defguard.gitbook.io/defguard/for-developers/contributing) for information on how to get started contributing to the project. You might also find our [environment setup guide](https://defguard.gitbook.io/defguard/for-developers/dev-env-setup) handy.

# Built and sponsored by

<p align="center">
      <a href="https://teonite.com" target="_blank"><img src="https://drive.google.com/uc?export=view&id=1z0fxSsZztoaeVWxHw2MbPbuOHMe3OsqN" alt="build by teonite" /></a>
</p>

# Legal
WireGuard® is [registered trademarks](https://www.wireguard.com/trademark-policy/) of Jason A. Donenfeld.
