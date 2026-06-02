<p align="center">
  <img src="docs/cover-image_smaller-logo.png" alt="defguard">
</p>

Defguard is a self-hosted secure remote access platform that combines WireGuard VPN, identity and access management, multi-factor authentication, and network access control in a single solution.

Built with a security-first architecture, Defguard helps organizations securely manage access to infrastructure, applications, and private networks while maintaining full control over their environment.

## Why Defguard?

Modern organizations often rely on multiple disconnected tools to manage identity, VPN access, authentication, and network permissions. Defguard brings these capabilities together into a unified platform designed for security, transparency, and operational simplicity.

Key principles behind Defguard:

- 📖 Open-source core (AGPL), open-code Enterprise components
- 🏠 Fully self-hosted — no external dependencies or data leaving your infrastructure
- 🔒 Security-first: [Zero-Trust VPN](https://docs.defguard.net/features/wireguard) with connection-level MFA, [architecture](https://docs.defguard.net/in-depth/architecture) designed to minimize attack surface
- 🔍 Transparency: [published SBOMs](https://defguard.net/sbom/), [penetration test reports](https://defguard.net/pentesting/), [architecture decision records](https://docs.defguard.net/in-depth/architecture-decision-records)

For detailed security information see the [secure-by-design documentation](https://docs.defguard.net/in-depth/secure-by-design).

## Core Capabilities

- 🌐 **WireGuard VPN** — multiple locations with per-location access control, MFA per connection, self-service device setup, kernel and userspace support
- 👥 **Identity & Access Management** — internal OIDC provider for SSO, external OIDC (Google, Microsoft, custom), LDAP/AD sync, remote enrollment, user self-service
- 🔑 **Multi-Factor Authentication** — TOTP, WebAuthn/FIDO2, email tokens, biometric via mobile app
- 🛡️ **Firewall** — allow/deny rules per VPN location by user or group, applied in real time
- 📋 **Activity Log** — audit log with filtering and search; real-time SIEM streaming (Enterprise)
- 🔗 **Integrations** — webhooks and REST API

## Clients

- 🖥️ **Desktop** (Linux, macOS, Windows) — VPN management with MFA, multi-instance and multi-location support, and real-time connection statistics. [Download](https://defguard.net/download/)
- 📱 **Mobile** (Android, iOS) — VPN management with MFA, QR code onboarding. [Android](https://play.google.com/store/apps/details?id=net.defguard.mobile) · [iOS](https://apps.apple.com/us/app/defguard-vpn-client/id6748068630)

## Architecture

Defguard follows a component-based architecture designed to reduce attack surface and support secure deployments.

<p align="center">
  <img src="docs/new_defguard-architecture.png" alt="architecture">
</p>

Strict division of responsibilities and network segmentation:
- **Core** — central management plane: identity, authentication, authorization, and policy
- **Edge** — public-facing entry point, exposes selected Defguard services [GitHub repo](https://github.com/DefGuard/proxy)
- **Gateway** — enforces network access policies for protected resources [GitHub repo](https://github.com/DefGuard/gateway)

For details refer to the [architecture documentation](https://docs.defguard.net/in-depth/architecture).

## Quick Start

The fastest way to evaluate Defguard is with the [one-line installer](https://docs.defguard.net/getting-started/one-line-install):

```bash
bash <(curl -sSL https://raw.githubusercontent.com/defguard/deployment/main/docker-compose2.0/setup.sh)
```

⚠️ Warning! This installation method is intended for testing, demonstrations, and evaluation purposes only. It is not recommended for production deployments. See the [deployment documentation](https://docs.defguard.net/deployment-strategies/overview) for production deployment guidance, architecture recommendations, and high-availability configurations.

## Documentation

Comprehensive documentation is available at: https://docs.defguard.net

## Video guides

Visit out YouTube channel to see our [video guides](https://www.youtube.com/playlist?list=PLVR33X0CUHUcoyLshs9S8VbsGgggouCAW).

## Community

We want to get as much feedback as possible, so we encourage you to:

- 💬 open a [GitHub discussion](https://github.com/DefGuard/defguard/discussions/new/choose)
- 🪲 report any missing [features](https://github.com/DefGuard/defguard/issues/new?assignees=&labels=feature&projects=&template=feature_request.md&title=) or [bugs](https://github.com/DefGuard/defguard/issues/new?assignees=&labels=bug&projects=&template=bug_report.md&title=) as issues

## Contributions

Please review the [Contributing guide](https://docs.defguard.net/for-developers/contributing) for information on how to get started contributing to the project. You might also find our [environment setup guide](https://docs.defguard.net/for-developers/dev-env-setup) handy.

## License
The code in this repository is available under a dual licensing model:

- Open Source License: The code, except for the contents of the "crates/defguard_core/src/enterprise" directory, is licensed under the AGPL license (see file LICENSE.md in this repository). This applies to the open core components of the software.
- Enterprise License: All code in this repository (including within the "crates/defguard_core/src/enterprise" directory) is licensed under a separate Enterprise License (see file crates/defguard_core/src/enterprise/LICENSE.md).

## Legal
WireGuard® is [registered trademarks](https://www.wireguard.com/trademark-policy/) of Jason A. Donenfeld.