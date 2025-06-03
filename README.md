<div align="center">
 <p align="center">
  Defguard is a <strong>true Zero-Trust WireGuard® VPN with 2FA/Multi-Factor Authentication</strong>, as each connection requires MFA (and not only when logging in into the client application like other solutions)
  <img width="1096" alt="zero-trust" src="https://github.com/user-attachments/assets/a3bed030-0d82-4f8c-9687-01cc5780eff7" />
  Our primary focus at defguard is on prioritizing security. Then, we aim to make this challenging topic both useful and as easy to navigate as possible.

[Website](https://defguard.net) | [Getting Started](https://docs.defguard.net/#what-is-defguard) | [Features](https://github.com/defguard/defguard#features) | [Roadmap](https://github.com/orgs/defguard/projects/5) | [Support ❤](https://github.com/defguard/defguard#support)

</div>

### Defguard provides Comprehensive Access Control (a complete security platform):

- **[WireGuard® VPN with 2FA/MFA](https://docs.defguard.net/admin-and-features/wireguard/multi-factor-authentication-mfa-2fa/architecture)** - not 2FA to "access application" like most solutions
    - The only solution with [automatic and real-time synchronization](https://docs.defguard.net/enterprise/automatic-real-time-desktop-client-configuration) for users' desktop client settings (including all VPNs/locations).
    - Control users [ability to manage devices and VPN options](https://docs.defguard.net/enterprise/behavior-customization)
- [Integrated SSO based on OpenID Connect](https://docs.defguard.net/admin-and-features/openid-connect): 
    - significant cost saving, simplifying deployment and maintenance
    - enabling features unavailable to VPN platforms relying upon 3rd party SSO integration
- Already using Google/Microsoft or other OpenID Provider? - [external OpenID provider support](https://docs.defguard.net/enterprise/external-openid-providers)
- Only solution with [secure remote user Enrollment & Onboarding](https://docs.defguard.net/help/enrollment)
- Yubico YubiKey Hardware [security key management and provisioning](https://docs.defguard.net/admin-and-features/yubikey-provisioning)
- Secure and robust architecture, featuring components and micro-services seamlessly deployable in diverse network setups (eg. utilizing network segments like Demilitarized Zones, Intranet with no external access, etc), ensuring a secure environment.
- Enterprise ready (multiple Locations/Gateways/Kubernetes deployment, etc..)
- Built on WireGuard® protocol which is faster than IPSec, and significantly faster than OpenVPN
- Built with Rust for speed and security

See:
- [full list of features](https://github.com/defguard/defguard#features)
- [enterprise only features](https://docs.defguard.net/enterprise/all-enteprise-features)

### Defguard makes it easy to manage complex VPN networks in a secure way

<img width="1564" alt="locations-connections" src="https://github.com/user-attachments/assets/f886750b-1d4e-467e-917d-bc19a86e275c" />

#### Video introduction

Bear in in mind we are no youtubers - just engineers - here is a video introduction to defguard:

<div align="center">
 <p align="center">
  
[![Introduction to defguard](https://img.youtube.com/vi/4PF7edMGBwk/hqdefault.jpg)](https://www.youtube.com/watch?v=4PF7edMGBwk)

</p>
</div>

### Control plane management (this video is few versions behind... - a lot has changed!)

![](https://defguard.net/images/product/core/hero-image.png)

![](https://github.com/DefGuard/docs/blob/docs/screencasts/defguard.gif?raw=true)

Better quality video can [be viewed here](https://github.com/DefGuard/docs/raw/docs/screencasts/defguard-screencast.mkv)

### Desktop Client with 2FA / MFA (Multi-Factor Authentication)

#### Light

![defguard desktop client](https://defguard.net/images/product/client/main-screen.png)

#### Dark

![defguard WireGuard MFA](https://github.com/DefGuard/docs/blob/docs/releases/0.9/mfa.png?raw=true)

[Desktop client](https://github.com/DefGuard/client):

- **2FA / Multi-Factor Authentication** with TOTP or email based tokens & WireGuard PSK
- [automatic and real-time synchronization](https://docs.defguard.net/enterprise/automatic-real-time-desktop-client-configuration) for users' desktop client settings (including all VPNs/locations).
- Control users [ability to manage devices and VPN options](https://docs.defguard.net/enterprise/behavior-customization)
- Defguard instances as well as **any WireGuard tunnel** - just import your tunnels - one client for all WireGuard connections
- Secure and remote user enrollment - setting up password, automatically configuring the client for all VPN Locations/Networks
- Onboarding - displaying custom onboarding messages, with templates, links ...
- Ability to route predefined VPN traffic or all traffic (server needs to have NAT configured - in gateway example)
- Live & real-time network charts
- live VPN logs
- light/dark theme

## Quick start

The easiest way to run your own defguard instance is to use Docker and our [one-line install script](https://docs.defguard.net/features/setting-up-your-instance/one-line-install).
Just run the command below in your shell and follow the prompts:

```bash
curl --proto '=https' --tlsv1.2 -sSf -L https://raw.githubusercontent.com/DefGuard/deployment/main/docker-compose/setup.sh -O && bash setup.sh
```

Here is a step-by-step video about this process:

<div align="center">
 <p align="center">
  
[![Quickly deploy defguard](https://img.youtube.com/vi/MqlE6ZTn0bg/hqdefault.jpg)](https://www.youtube.com/watch?v=MqlE6ZTn0bg)

</p>
</div>

To learn more about the script and available options please see the [documentation](https://docs.defguard.net/features/setting-up-your-instance/one-line-install).

### Setup a VPN server in under 5 minutes !?

Just follow [this tutorial](http://bit.ly/defguard-setup)

## Manual deployment examples

- [Standalone system package based install](https://docs.defguard.net/admin-and-features/setting-up-your-instance/standalone-package-based-installation)
- Using [Docker Compose](https://docs.defguard.net/features/setting-up-your-instance/docker-compose)
- Using [Kubernetes](https://docs.defguard.net/features/setting-up-your-instance/kubernetes)

## Roadmap & Development backlog

[A detailed product roadmap and development status can be found here](https://github.com/orgs/DefGuard/projects/5/views/1)

### ⛑️ Want to help? ⛑️

Here is a [dedicated view for **good first bugs**](https://github.com/orgs/DefGuard/projects/5/views/5)

## Why?

The story and motivation behind defguard [can be found here: https://teonite.com/blog/defguard/](https://teonite.com/blog/defguard/)

## Features

* Remote Access: [WireGuard® VPN](https://www.wireguard.com/) server with:
  - [Multi-Factor Authentication](https://docs.defguard.net/help/desktop-client/multi-factor-authentication-mfa-2fa) with TOTP/Email & Pre-Shared Session Keys
  - multiple VPN Locations (networks/sites) - with defined access (all users or only Admin group)
  - multiple [Gateways](https://github.com/DefGuard/gateway) for each VPN Location (**high availability/failover**) - supported on a cluster of routers/firewalls for Linux, FreeBSD/PFSense/OPNSense
  - **import your current WireGuard® server configuration (with a wizard!)**
  - **most beautiful [Desktop Client!](https://github.com/defguard/client)** (in our opinion ;-))
  - automatic IP allocation
  - [automatic and real-time synchronization](https://docs.defguard.net/enterprise/automatic-real-time-desktop-client-configuration) for users' desktop client settings (including all VPNs/locations).
  - control users [ability to manage devices and VPN options](https://docs.defguard.net/enterprise/behavior-customization)
  - kernel (Linux, FreeBSD/OPNSense/PFSense) & userspace WireGuard® support with [our Rust library](https://github.com/defguard/wireguard-rs)
  - dashboard and statistics overview of connected users/devices for admins
  - *defguard is not an official WireGuard® project, and WireGuard is a registered trademark of Jason A. Donenfeld.*
* Identity & Account Management:
  - SSO based on OpenID Connect](https://openid.net/developers/how-connect-works/)
  - External SSO: [external OpenID provider support](https://docs.defguard.net/enterprise/external-openid-providers)
  - [Multi-Factor/2FA](https://en.wikipedia.org/wiki/Multi-factor_authentication) Authentication:
   - [Time-based One-Time Password Algorithm](https://en.wikipedia.org/wiki/Time-based_one-time_password) (TOTP - e.g. Google Authenticator)
   - WebAuthn / FIDO2 - for hardware key authentication support (eg. YubiKey, FaceID, TouchID, ...)
   - Email based TOTP
  - LDAP (tested on [OpenLDAP](https://www.openldap.org/)) synchronization
  - [forward auth](https://docs.defguard.net/features/forward-auth) for reverse proxies (tested with Traefik and Caddy)
  - nice UI to manage users
  - Users **self-service** (besides typical data management, users can revoke access to granted apps, MFA, WireGuard®, etc.)
* Account Lifecycle Management:
  - Secure remote (over the Internet) [user enrollment](https://docs.defguard.net/help/remote-user-enrollment) - on public web / Desktop Client
  - User [onboarding after enrollment](https://docs.defguard.net/help/remote-user-enrollment/user-onboarding-after-enrollment)
* SSH & GPG public key management in user profile - with [SSH keys authentication for servers](https://docs.defguard.net/admin-and-features/ssh-authentication)
* [Yubikey hardware keys](https://www.yubico.com/) provisioning for users by *one click*
* [Email/SMTP support](https://docs.defguard.net/help/setting-up-smtp-for-email-notifications) for notifications, remote enrollment and onboarding
* Easy support with [sending debug/support information](https://docs.defguard.net/help/sending-support-info)
* Webhooks & REST API
* Built with [Rust](https://www.rust-lang.org/) for portability, security, and speed
* [UI Library](https://github.com/defguard/ui) - our beautiful React/TypeScript UI is a collection of React components:
  - a set of custom and beautiful components for the layout
  - Responsive Web Design (supporting mobile phones, tablets, etc..)
  - [iOS Web App](https://www.macrumors.com/how-to/use-web-apps-iphone-ipad/)
* **Checked by professional security researchers** (see [comprehensive security report](https://defguard.net/pdf/isec-defguard.pdf))
* End2End tests

## Documentation

See the [documentation](https://docs.defguard.net/) for more information.

## Community and Support

Find us on Matrix: [#defguard:teonite.com](https://matrix.to/#/#defguard:teonite.com)

## License

The code in this repository is available under a dual licensing model:

1. Open Source License: The code, except for the contents of the "crates/defguard_core/src/enterprise" directory, is licensed under the AGPL license (see file LICENSE.md in this repository). This applies to the open core components of the software.
2. Enterprise License: All code in this repository (including within the "crates/defguard_core/src/enterprise" directory) is licensed under a separate Enterprise License (see file crates/defguard_core/src/enterprise/LICENSE.md).

## Contributions

Please review the [Contributing guide](https://docs.defguard.net/for-developers/contributing) for information on how to get started contributing to the project. You might also find our [environment setup guide](https://docs.defguard.net/for-developers/dev-env-setup) handy.

# Built and sponsored by

<p align="center">
      <a href="https://teonite.com/services/rust/" target="_blank"><img src="https://drive.google.com/uc?export=view&id=1z0fxSsZztoaeVWxHw2MbPbuOHMe3OsqN" alt="built by teonite" /></a>
</p>

# Legal

WireGuard® is [registered trademarks](https://www.wireguard.com/trademark-policy/) of Jason A. Donenfeld.
