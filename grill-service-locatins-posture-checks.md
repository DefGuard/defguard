# Posture checks for service locations — design session

Record of a design interview covering the client, core, proxy and shared-proto repos, plus the
resulting implementation plan.

- **Repos:** `client/src-tauri`, `defguard` (core + `web/`), `proxy`, and the shared `proto` submodule
- **Status:** design agreed; not yet implemented. Two process questions still open (see §10).

---

## 1. The problem

**Service locations** are background WireGuard tunnels managed by the privileged system service
(`defguard-service`), brought up at boot with no logged-in user. **Posture-gated locations** require the
client to POST device posture signals to the proxy and receive a preshared key (PSK) that the gateway
demands before it will add the peer.

Today the two are mutually exclusive, in three separate ways:

1. **Core forbids it.** `set_postures_for_location` / `set_locations_for_posture` return 400
   *"Posture checks cannot be assigned to service locations"*
   (`crates/defguard_core/src/enterprise/handlers/device_posture.rs:1166-1170`, `:1237-1245`), and the
   web UI hides the posture step for service locations.
2. **The client would silently fail anyway.** `Location::is_service_location()`
   (`core/src/database/models/location.rs:549-554`) ignores `posture_check_required`, so such a location
   is pushed to the daemon and connected **with no PSK**. The gateway refuses a peer without a live
   posture session, so the tunnel comes up locally and never passes traffic.
3. **The service structurally cannot do posture.** `SaveServiceLocationsRequest` carries only
   `instance_id` + `private_key` + a flattened location list — no `proxy_url`, no device pubkey, no
   `network_id`.

**Goal:** let an admin assign posture checks to a service location, and have the system service
authorize itself at boot — before/without any user session — and keep that authorization alive.

---

## 2. Findings that reshaped the original plan

The starting proposal was: store `proxy_url`, a **token**, and a posture flag in the service-location
JSON; refactor so posture checks can run from the service; authorize at service startup. Three
investigations (client subsystems, core protocol, proxy endpoint) changed two of its premises.

### 2.1 The posture endpoint has no authentication — the token is unnecessary

`POST <proxy_url>/api/v1/posture/connect` has no `Authorization` header, no cookie, no mTLS, no nonce.
The proxy handler (`proxy/src/enterprise/handlers/desktop_client_posture.rs`) is a pure pass-through to
core over the bidi gRPC stream. Core's `handle_posture_check`
(`crates/defguard_core/src/grpc/proxy/client_mfa.rs:865-1008`) resolves identity entirely from the
request body:

```rust
let Ok(Some(location)) = WireguardNetwork::find_by_id(&self.pool, request.location_id).await else { ... };
if location.mfa_enabled() { /* refuse posture-only sessions on MFA locations */ }
let Ok(Some(device)) = Device::find_by_pubkey(&self.pool, &request.pubkey).await else { ... };
// then: has_postures, user exists, user.is_active, validate_location_access (group membership)
```

The `defguard-client-version` / `defguard-client-platform` headers are read into `DeviceInfo` for
telemetry and **never validated** — and core discards `device_info` entirely on this path.

**Consequence:** the service needs **no secret at all**, only:

| Input | Source |
|---|---|
| `proxy_url` | `Instance.proxy_url` |
| `pubkey` | `WireguardKeys.pubkey` — the device's **public** key |
| `location_id` | `Location.network_id` — core's `WireguardNetwork` id |
| posture data | gathered locally |

This is strictly better than the original plan: no long-lived token lands in a file on disk, so a
root/SYSTEM compromise gains nothing beyond the WireGuard private key already stored there.

⚠️ Note `ServiceLocation.pubkey` in the existing proto is the **remote/server** key, not the device key.
These must not be conflated — hence a separate `device_pubkey` field.

### 2.2 Core actively blocks the combination — so core changes are unavoidable

Beyond the two guards, the investigation found **three** inconsistencies:

- `create_network` (`crates/defguard_core/src/handlers/wireguard.rs:268-284`) has **no** posture guard,
  so the combination is already reachable there.
- `modify_network` (`:336-429`) never reads `data.posture_checks`, so flipping a posture-carrying regular
  location into a service location silently leaves the postures attached.
- `create_network` also permits `location_mfa_mode != Disabled` **together with**
  `service_location_mode != Disabled`. `modify_network` silently downgrades this (`:388-396`). Such a
  location is invisible on the client — `is_service_location()` is false *and*
  `get_service_location_mode_filter(false)` excludes it from the app's list.

So the combination is currently reachable only via inconsistent API paths. Formalising it means fixing
all three.

### 2.3 Three server behaviours that constrain the design

1. **Re-authorization is destructive.** `create_new_session` (`client_mfa.rs:1012-1066`) disconnects the
   prior session for `(location, device)` as `Superseded`, deletes the gateway peer, and re-adds it with
   the new PSK. So a *healthy* tunnel must never be re-authorized.
2. **Authorization expires.** A session is revoked if no handshake occurs within
   `peer_disconnect_threshold` (default 300s) of creation, or after that long idle
   (`crates/defguard_session_manager/src/lib.rs:207-282`). Keepalive is 25s, so a healthy tunnel never
   trips it — but suspend/resume or a server-side revocation will.
3. **Posture fails closed.** `InsufficientPermissions` / `DetectionFailed` / an absent field all become
   `CheckUnavailable` → 403 (`crates/defguard_core/src/enterprise/posture/evaluation.rs:21-70`).

### 2.4 The PSK is not really a "preshared key"

Core generates a **fresh keypair per call** and returns its *public* half as the PSK
(`let key = WireguardNetwork::genkey(); ... preshared_key: key.public`), storing it on a
`vpn_client_session` row. It is a per-session, per-`(location, device)` secret — not a stable device
credential.

---

## 3. The design session

Each round records the question, the decision, and the evidence that drove it. Rounds 3 onward are
grounded in the investigation findings.

### Round 1 — framing

| Question | Decision | Rationale |
|---|---|---|
| Where should the token live, given the JSON is `0600` root / SYSTEM+Admins ACL? | *(superseded in Round 3)* | Initially "store it in the JSON", on the grounds that the file already holds the WireGuard private key so the marginal risk is low. Overturned once the endpoint was found to need no token at all. |
| Should posture gathering + the proxy call live in the service or the app? | **Service owns it end-to-end; app calls via IPC** | One implementation, one code path. Matches the direction Windows already took (posture is already gathered in the daemon there because BitLocker detection needs SYSTEM). It is also the only shape that works with no user logged in. |
| Failure policy on posture failure at startup? | *(deferred, answered below)* | |
| PSK lifetime handling? | *(deferred pending server-side facts)* | Deliberately not guessed — the answer depends on whether the PSK is permanent, session-scoped, or expiring, which was unknown at that point. |

Clarification given mid-round: **retry, but without backoff — a regular connect loop.**

### Round 2 — retry shape and platform reach

| Question | Decision | Rationale |
|---|---|---|
| The existing loop is bounded (5 attempts × 30s, then gives up). What should "regular connect loop" mean? | **Unbounded, fixed interval** | An always-on tunnel that permanently gives up is a worse failure than a chatty retry. |
| Which platforms move to the IPC model? | **Windows + Linux only; macOS stays in-process** | Service locations don't exist on macOS at all (`ClientFeature::ServiceLocations` has no macOS rule, and there is no macOS implementation in the service-locations crate), so moving macOS posture into its daemon would be pure cost. |
| PreLogon + posture? | *(deferred)* | |

### Round 3 — grounded in the investigation

| Question | Decision | Rationale |
|---|---|---|
| What goes in the JSON, now that no token is needed? | **`proxy_url` + `device_pubkey` per instance; `network_id` + `posture_check_required` per location** | Exactly what the proxy call needs and nothing more. No new secret on disk. |
| Core blocks the combination. What's the scope? | **Include core + web changes** | Without them an admin literally cannot configure the feature, so the client work would be untestable end-to-end. |
| How should re-authorization be triggered? | **Only when the tunnel is actually dead/stale** (plus network change, resume) | Because re-auth supersedes the session and bounces the gateway peer, a periodic re-auth would deliberately kill a healthy tunnel every interval. With keepalive at 25s a healthy tunnel never trips the 300s threshold. |
| The Linux disk probe derives its target from `db_file_path()` (per-user). What happens when run as root? | **Pass the target path in as a parameter** | Run as root, `db_file_path()` resolves to root's `$XDG_DATA_HOME` — a different, possibly differently-encrypted partition. Also removes the last `core::database` dependency from `inspector`. |

### Round 4 — mechanics

| Question | Decision | Rationale |
|---|---|---|
| Adding JSON fields is a breaking change (plain serde, no defaults, no version). | **`#[serde(default)]` on all new fields** | Missing keys then reproduce today's behaviour exactly for pre-upgrade files. Since posture-gated service locations cannot predate the feature, nothing needs backfilling. |
| What shape should the new RPC take? | **`AuthorizePostureSession(proxy_url, device_pubkey, location_id) -> preshared_key`** | The caller supplies the three inputs, so the same function serves both the app (for *regular* posture locations, which have no JSON entry) and the service's own boot path. One implementation. |
| `save_service_locations` resets **every** location on every sync. | **Reset only locations whose config actually changed** | With posture, an unconditional reset means a fresh `posture/connect` per location per sync — superseding the session, deleting and re-adding the gateway peer. Healthy tunnels would bounce on every config change. |
| PreLogon + posture? | *(deferred again)* | |

### Round 4b — "which Windows signals are hard to check before logon?"

Asked directly, with the posture-policy UI as context. Answer, from the probe implementations:

| UI option | Probe | Pre-logon risk |
|---|---|---|
| Windows version / Any version | `sysinfo::System::os_version()` / `name()` | **None** — registry-backed |
| Windows security updates | WMI `Win32_QuickFixEngineering` (`inspector/windows.rs:167-180`) | **Low** — needs `Winmgmt` up; the class is also notoriously slow |
| Connected to Active Directory | WMI `Win32_ComputerSystem.PartOfDomain` (`:154-158`) | **None** — it's whether the *computer* is domain-joined, not whether a domain user is signed in |
| Antivirus installed | WMI `root\SecurityCenter2` `AntiVirusProduct`, cross-checked against Defender `MSFT_MpComputerStatus` (`:126-145`) | **Highest** — Security Center (`wscsvc`) is Automatic **(Delayed Start)**, so early in boot the namespace can return no products |
| Disk encryption enabled | WMI `root\CIMV2\Security\MicrosoftVolumeEncryption` (`:73-98`) | **None — and better pre-logon.** Admin-only namespace; this is the whole reason posture already runs in the SYSTEM service on Windows. BitLocker is necessarily unlocked by the time Windows boots. |

Two conclusions:

1. **None of the signals are user-scoped** — all five are machine-scoped. The real risk is *early boot
   timing*, not the absence of a user session. An earlier hesitation about "are pre-logon signals
   meaningful" conflated the two.
2. **Therefore this cannot distinguish PreLogon from Always-on**, since both connect at system boot and
   face the identical window. Boot-timing failures fail closed (403), and the unbounded 30s retry is what
   recovers them — harmlessly, since a failed check creates no session.

### Round 5 — PreLogon resolved

| Question | Decision | Rationale |
|---|---|---|
| PreLogon + posture on Windows? | **Allow both modes on Windows** | Signal availability doesn't differentiate them. The logon/logoff watcher already tears down and rebuilds PreLogon tunnels; each rebuild simply re-authorizes. Linux stays Always-on-only because it filters PreLogon out at save time regardless. |
| Retry interval? | **Keep 30s** | Recovers a delayed Security Center within a minute or two. The proxy has no rate limiting by default, so a permanently-rejected device polls indefinitely at this rate — acceptable, and it means remediation is picked up with no user action. |

### Round 6 — the machinery

| Question | Decision | Rationale |
|---|---|---|
| Nothing currently detects staleness. How should it be detected? | **New periodic health task in the daemon, both platforms** | Windows' `NotifyAddrChange` watcher only calls `connect_to_service_locations`, which skips anything already in its in-memory map regardless of whether handshakes are flowing. The app's `verify_active_connections` doesn't cover service locations. A handshake-based check is the only mechanism that notices **server-side revocation**, which fires no event. |
| The MFA path needs *raw* signals, not authorization. What should Linux do? | **Implement `GetPostureData` on Linux too; app always uses IPC** | One collection point per platform. Required anyway once the disk probe takes an explicit path — otherwise app and service could report different disk-encryption values for the same machine. |
| How to distinguish rejection from transient failure over IPC? | **Distinct tonic status codes** | Rejection → `permission_denied` carrying the reason string; transient → `unavailable`/`internal`. The app maps `permission_denied` back to `ConnectError::PostureCheckFailed`, so the existing UI path is unchanged. |
| Does the combination need its own client-version gate? | **No — ships within 2.1** | Every 2.1.0 client will understand it, so the existing `ServiceLocations` + `PostureChecks` rules suffice. |

### Round 7 — scope boundaries

| Question | Decision | Rationale |
|---|---|---|
| Health task timings? | **Check every 60s; stale after 180s; as constants** | 180s is far above the 25s keepalive (no false positives) and safely under core's 300s default, so the client re-authorizes *before* core revokes. |
| How far should the crate restructuring go? | **Also split `inspector` into its own crate** | Chosen over the narrower "decouple the code path only" option. The daemon then links only what it needs and stops pulling `core`/`sqlx`; it does now legitimately need an HTTP client. |
| Should the CLI move to IPC too? | **Yes, on Windows + Linux** | It already holds a `DAEMON_CLIENT` and runs on exactly those platforms. Otherwise the CLI would lose the BitLocker signal on Windows entirely (admin-only WMI) and could disagree with the service about the same machine. |
| Add audit events for posture-only connects? | **Yes** | `handle_posture_check` emits nothing today, unlike the MFA path. With authorization happening headlessly at boot there would be no audit trail at all. The event types already exist. |

Directive given mid-round, on user-facing feedback:

> only emit appropriate error log to the service logfile, don't do any log watching/notifications.

### Round 8 — licensing

| Question | Decision | Rationale |
|---|---|---|
| A lapsed `DevicePosture` licence makes `validate_posture` return `NoActiveEnterpriseLicense`, which core converts to `PostureResult::Pass` — so a PSK is still minted and an always-on tunnel keeps running unenforced. Meanwhile a lapsed `ServiceLocations` licence yields zero gateway peers. | **Require both; keep existing lapse semantics** | Don't change licence behaviour in this task. The degradation is exactly what already happens for regular posture locations. Any change is a product-level decision. |

### Round 9 — final holes

Surfaced by the design pass; none were covered by the decisions above.

| Question | Decision | Rationale |
|---|---|---|
| Unassigning postures from a live service location bricks it until someone logs in: core returns 400 *"location does not use postures"*, but the daemon's JSON still says `posture_check_required: true`, so fail-closed retries forever — and on a headless pre-logon box the app never runs to fix the JSON. | **Core returns `Approved { preshared_key: "" }` (no session) when `!has_postures`; client treats an empty PSK as "connect without one"** | Correct precisely because `get_location_allowed_peers` then offers the gateway `preshared_key: None`, so a PSK-configured client could never handshake anyway. Backward-compatible for the app path, and it self-heals headlessly. The rejected alternative — having the client infer this from the error — is dangerous, because that error maps to the same class as "device not found" / "user is inactive", where connecting without a PSK would be wrong. |
| On a definitive 403 for an **already-up** tunnel: tear down or leave up? | **Tear the interface down** | Matches fail-closed and makes the failure visible, rather than leaving a tunnel that blackholes everything in `allowed_ips`. Core revokes ~300s later anyway, so this mainly makes the client agree with the server sooner. |
| `peer_disconnect_threshold` is admin-configurable and the daemon can't learn it, so 180s could be too slow. | **Ship the 180s constant; document the assumption** | No new proto field. A deployment lowering it below 180s gets server-side revocation first — recoverable on the next tick, at the cost of a brief outage. |

---

## 4. Decision register

| # | Decision |
|---|---|
| D1 | JSON gains per-instance `proxy_url` + `device_pubkey`, per-location `network_id` + `posture_check_required`. **No token.** New fields `#[serde(default)]` **per-field, not container-level**. |
| D2 | Service owns posture end-to-end on **Windows + Linux**; macOS keeps in-process. |
| D3 | New RPC `AuthorizePostureSession(proxy_url, device_pubkey, location_id) -> {preshared_key}`. Keep `GetPostureData`, **implement it on Linux too**. |
| D4 | Fail closed — never connect without a PSK. **Unbounded fixed-interval retry at 30s**, no backoff. |
| D5 | Re-authorize only when dead/stale, on network change, or on resume. Never on a healthy timer. |
| D6 | Periodic health check in the daemon, both platforms: every **60s**, stale after **180s** without a handshake. |
| D7 | `save_service_locations` acts **only on locations whose config changed**. |
| D8 | Core + web in scope: lift the guard, close the `create_network`/`modify_network` inconsistencies, expose in the admin UI. |
| D9 | Linux disk-encryption probe takes the target path as a **parameter**. |
| D10 | **Split `inspector` into its own crate**; daemon stops linking `core`/`sqlx`. |
| D11 | IPC errors: rejection → `permission_denied`; transient → `unavailable`; bad request → `failed_precondition`. |
| D12 | PreLogon + posture allowed on **Windows**; Linux stays Always-on-only. |
| D13 | Ships within 2.1 → no new `ClientFeature` rule. |
| D14 | `client-cli` also uses IPC on Windows + Linux. |
| D15 | Add `PostureCheckPassed` / `PostureCheckFailed` events to core's posture-only path. |
| D16 | `authorize_posture_session` takes `(proxy_url, device_pubkey, location_id)` — no DB access. |
| D17 | **No user-facing notification.** Log to the service logfile only; admin visibility via D15. |
| D18 | Require both licences; keep existing lapse semantics. |
| D19 | Core returns `Approved { preshared_key: "" }` (no session) when `!has_postures`; client treats an empty PSK as "connect without one". |
| D20 | A definitive 403 on re-authorizing an **already-up** tunnel **tears the interface down**. |
| D21 | Ship the 180s staleness threshold as a constant; document the `peer_disconnect_threshold >= 300s` assumption. No new proto field. |

---

## 5. Implementation plan

### Repos and ordering

`src-tauri/proto` is a **git submodule** (`../proto.git`) shared by all three repos, pinned at the same
commit — but **only the client compiles `v1/client/client.proto`**, so the proto change blocks the client
only. **Proxy needs zero changes** (its handler is a pure pass-through).

Landing order: `1 ‖ 2` → `3` → `4` → `5` → `6` → `7` → `8` → `9 ‖ 10` → `11`

Hard constraints: core must permit the combination before anything is testable (P1); the proto PR and
submodule bump must precede client P4–P8 (P3); the crate split should precede new posture code so it is
written once against the final layout (P2); D7's enriched state record is a prerequisite for D6 (P5
before P8); the lock restructuring must land with the first `await` in the manager path or it **will not
compile** (guards are `!Send`) — fold into P5.

### Phase 1 — core: lift the guard, close three inconsistencies

Files: `crates/defguard_core/src/enterprise/handlers/device_posture.rs`,
`crates/defguard_core/src/handlers/wireguard.rs`, integration tests.

1. Delete both guards (`device_posture.rs:1166-1170`, `:1237-1245`).
2. `create_network` (`:268-284`) has no posture guard; `modify_network` (`:336-429`) never reads
   `data.posture_checks`. Make both consistent.
3. Add a shared `validate_service_location_mfa()` called from both, and **replace the silent
   MFA-downgrade at `:388-396` with a 400** — silently discarding an admin's choice is worse.
4. Sequencing: `get_location_allowed_peers` branches on `has_postures`
   (`location_management/allowed_peers.rs:30-31`), so in `modify_network` the posture write must happen
   **before** the `peers` / `maybe_firewall_config` computation at `:405-407`.
5. Invert the two tests asserting 400 (`tests/integration/api/device_posture.rs:1305-1411`); add one
   asserting `create_network` with MFA+service returns 400.
6. **D19:** change `handle_posture_check` (`client_mfa.rs:903-912`) to return
   `Approved { preshared_key: String::new() }` **without creating a session** when `!has_postures`. Add a
   test asserting no `vpn_client_session` row is created.

**Safe alone:** the runtime authorization machinery already keys off `has_postures` with no
service-location special case, so P1 by itself converts today's *silent* failure into a *loud* one — the
correct intermediate state.

### Phase 2 — client: crate split (D10, D9), mechanical

**New `enterprise/posture-inspector`** — today's `enterprise/posture/src/inspector/**`, depending only on
`common` + `client-proto` (+ per-OS `sysinfo`/`wmi`/`time`). Sever two couplings:

- `inspector/mod.rs:12` `core::version::PKG_VERSION` → `defguard_client_common::VERSION`
- `inspector/linux.rs:8,45` `db_file_path()` → **pass the path in** (D9):
  `disk_encryption_status(target: &Path)`, `device_posture_data(disk_probe_target: &Path)`, plus
  `pub const SYSTEM_VOLUME_PROBE_PATH: &str = "/"`. Windows/macOS impls ignore the argument, so there is
  one signature and no `cfg` at call sites.

**Move the HTTP helpers into `common`** (`common/src/http.rs`) — `post_with_headers`,
`construct_platform_header`, `CLIENT_*_HEADER`, `HTTP_REQ_TIMEOUT`, moved verbatim from
`core/src/proxy.rs`. Leave `core/src/proxy.rs` as a `pub use` shim so existing MFA/enrollment call sites
are untouched. This is what lets `enterprise/posture` drop its `core` dependency.

**New `enterprise/posture/src/error.rs`** — a **3-way** split, which is what makes D11 implementable:

```rust
pub enum PostureError {
    Rejected(String),       // 403: core evaluated and said no. Comma-joined reasons.
    Unavailable(String),    // network/timeout/5xx/429/503. Retryable.
    InvalidRequest(String), // 4xx that isn't 403 ("device not found", "user is inactive", ...)
    InvalidProxyUrl(String),
}
```

Also update the **six CI jobs** in `client/.github/workflows/posture.yaml` that reference
`-p defguard-client-posture --lib inspector::tests::ci::<os>::setupN`, plus `deny.toml` and
`[workspace].members`.

**Acceptance criterion for D10:** `cargo tree -p defguard-client-service | grep -c sqlx` == 0.

### Phase 3 — proto: new fields + RPC (shared submodule)

```protobuf
// NOTE: numbering is intentionally INCOMPATIBLE with common/client_types.proto's
// ServiceLocationMode (DISABLED=1/PRELOGON=2/ALWAYSON=3). Do NOT renumber: these
// values are persisted as integers in the daemon's on-disk JSON by clients >= 1.6.
message ServiceLocation {              // existing 1-8 unchanged
  int64 network_id = 9;                // core's WireguardNetwork id = posture location_id
  bool posture_check_required = 10;
}
message SaveServiceLocationsRequest {  // existing 1-3 unchanged
  string proxy_url = 4;                // Instance.proxy_url
  string device_pubkey = 5;            // WireguardKeys.pubkey — the DEVICE key core matches on
}
message AuthorizePostureSessionRequest  { string proxy_url = 1; string device_pubkey = 2; int64 location_id = 3; }
message AuthorizePostureSessionResponse { string preshared_key = 1; }

service DesktopDaemonService { /* +1 */ rpc AuthorizePostureSession(...) returns (...); }
```

**`client-proto/build.rs`:** `.skip_debug(["AuthorizePostureSessionResponse"])` so a live PSK cannot reach
a log via prost's derived `Debug`; `#[serde(default)]` as **per-field** attributes on the two new
`ServiceLocation` fields. Container-level `default` would let a truncated file deserialize as "zero
locations", which under D7 means *disconnect everything*.

### Phase 4 — client: JSON schema + both push sites (D1)

`ServiceLocationData` gains `#[serde(default)] proxy_url`, `device_pubkey`, `schema_version`
(+ `SERVICE_LOCATION_SCHEMA_VERSION: u32 = 1`). Extend the manual `Debug` for the new (non-secret)
fields; keep masking `private_key`.

**Single-source the mode mapping** — `proto_service_location_mode(...)` as the only place the DB enum
(1/2/3) maps to the wire enum (0/1), plus a unit test pinning **both** sets of numeric discriminants so a
future renumber breaks the build.

**Collapse both push sites** into `build_save_service_locations_request(instance, keys, locations)`:

| Field | Site A `src/commands.rs:460-510` | Site B `config-sync/commands.rs:154-260` |
|---|---|---|
| `network_id`, `posture_check_required` | `location.*` | same |
| `proxy_url` | `instance.proxy_url` (in scope) | `instance.proxy_url` (freshly assigned at `:60`) |
| `device_pubkey` | `keys.pubkey` (row fetched; only `prvkey` used today) | `WireguardKeys::find_by_instance_id(...).pubkey` (row already fetched at `:214`) |

Two fixes while here:

- Site A's early-return at `:480` means an instance whose service locations all disappeared never gets
  `DeleteServiceLocations`. Add the `else` branch mirroring `sync_service_locations:176-203`.
- **Site B signature → `sync_service_locations(pool: &DbPool, instance: &Instance<Id>)`**, called
  **after** `transaction.commit()` and **unconditionally**. Unconditional matters because
  `locations_changed` compares `Location` sets and therefore **does not notice a `proxy_url` change** —
  which D1 now puts in the JSON. Safe once D7 makes it a no-op.

### Phase 5 — client: change detection (D7) + lock discipline

**`ConnectedServiceLocation`** replaces the bare `ServiceLocation` in `connected_service_locations`:
`location`, `ifname` (**must be recorded** — Linux `get_interface_name` allocates the next free `wgN`, so
it cannot be recomputed), `listen_port` (reused across re-auth so NAT bindings survive), `preshared_key`
(**in memory only, never persisted**), `authorized_at`, `last_rx_bytes`, `reauth_pending_since`,
`last_rejection`. This also removes the existing `find_interface_by_peer_pubkey` syscall scan
(`linux.rs:186-212`).

**`diff.rs`** — pure, unit-tested, no I/O. Two fingerprints over incoming vs stored:

- `TunnelFingerprint` (name/address/pubkey/endpoint/allowed_ips/keepalive/dns/mode) → differs ⇒ **Rebuild**
- `AuthFingerprint` (network_id/posture_check_required/proxy_url/device_pubkey) → differs ⇒ **Reauthorize**
- both equal ⇒ **Unchanged**; absent before ⇒ **Add**; `private_key` changed ⇒ everything **Rebuild**

Identity is `network_id`, with a pubkey fallback for pre-2.1 files where it defaults to 0.
**Upgrade case:** a pre-2.1 file yields `Unchanged` for non-posture locations, so an in-place client
upgrade **does not flap healthy tunnels**.

`save_service_locations(&mut self, request) -> Result<ServiceLocationDiff>` now performs **no network
I/O**: it writes the file, disconnects removed pubkeys, handles non-posture Add/Rebuild inline, and
returns the diff for the async reconciler.

**Lock discipline** — add `with_manager_read/write(manager, closure)` helpers taking **non-async**
closures on `tokio::task::spawn_blocking`, so it is *structurally impossible* to `await` under the
`std::sync` guard. Use `unwrap_or_else(PoisonError::into_inner)` throughout — today one panic under the
lock poisons it and every later `.unwrap()` aborts the daemon, killing the stats streams the app depends
on. Convert `daemon.rs:217,256`, `lib.rs:151`, `windows.rs:73-76,118-128`.

### Phase 6 — client: the RPC (D3, D11) + close the contention hole

- `daemon/src/posture.rs`: `authorize_posture_session(...)` (the one daemon-side path, called by both the
  RPC handler and the reconciler) and `posture_error_to_status` for D11.
- `daemon/src/daemon.rs`: add the `AuthorizePostureSession` handler; change `get_posture_data`'s cfgs from
  `windows`/`not(windows)` to `any(windows, linux)`/`macos` (which also removes the pre-existing inverted
  log/status text at `:240-245`). Linux passes `SYSTEM_VOLUME_PROBE_PATH`.
- `core/src/posture_ipc.rs`: the IPC shims. Map `Code::PermissionDenied` → `Error::PostureCheckFailed` so
  `ConnectError::from` (`src/commands.rs:80-92`) and the frontend need **no change**. Handle
  `Code::Unimplemented` for a daemon older than the app.
- **SSRF hardening:** this RPC lets any `defguard`-group member make the **root/SYSTEM** daemon POST the
  machine's privileged posture data to an **arbitrary URL**. Require `https` (allow `http` for localhost
  only under `debug_assertions`), set `redirect::Policy::none()`, keep the 5s timeout, log the target
  host. Do *not* whitelist against the JSON — the app legitimately needs this for regular posture
  locations the daemon has no stored `proxy_url` for.
- **Close the contention path:** reject `location.is_service_location()` in `src/commands.rs::connect`
  (`:146`) and `client-cli/src/resolve.rs:34-38`. Both use `Location::find_by_id`, which is a plain
  `WHERE id = $1` (`location.rs:235-251`) and does **not** filter service locations — so
  `dg connect --id <service-location-id>` supersedes the service's session and builds a second interface
  for the same peer. `find_by_name` already filters.

### Phase 7 — client: reconciler replaces the bounded retry loop (D4)

Delete `connect_service_locations` (`lib.rs:135-171`) and the `SERVICE_LOCATION_CONNECT_RETRY_*`
constants. One task owns all state transitions on both platforms:

```rust
SERVICE_LOCATION_RECONCILE_INTERVAL        = 30s   // D4, unbounded, no backoff
SERVICE_LOCATION_HEALTH_CHECK_INTERVAL     = 60s   // D6
SERVICE_LOCATION_STALE_HANDSHAKE_THRESHOLD = 180s  // D6
POSTURE_MIN_KEEPALIVE_INTERVAL             = 25    // keepalive floor
run_service_location_reconciler(manager, wake: Arc<Notify>)
enum ReconcileTrigger { Tick, Save, NetworkChange, LoginLogoff, Resume, Startup }
```

Per pass: read-lock snapshot → for each location needing work, **authorize first, mutate second** (the key
guard: no network ⇒ `Unavailable` ⇒ record `reauth_pending_since` and *never touch the interface*) →
staleness scan only on `Tick`, at most every 60s.

Rewire the Windows watchers to `wake.notify_one()` instead of calling into the manager from an OS thread
under the write lock (`windows.rs:73-76`, `118-128`); keep the logon *disconnect* immediate since it is
pure local teardown. Add `ServiceControlAccept::POWER_EVENT` and handle
`PowerEvent(ResumeSuspend | ResumeAutomatic)` → `wake`. Linux has no resume hook — the 30s tick is the
interim.

**Mode-0 hazard:** `PRE_LOGON = 0` is the proto3 default, and Linux filters `mode != AlwaysOn`
(`linux.rs:84-88`). A location whose mode failed to serialize is silently dropped *and*, under D7,
disconnected as removed. Add a `warn!` naming the filtered mode, plus a `to_service_location` round-trip
test over all three DB modes.

### Phase 8 — client: health check (D5, D6) — inside the reconciler

`assess_health(&self, stale_threshold, now) -> Vec<(instance_id, network_id, LocationHealth)>` where
`LocationHealth ∈ {Healthy, Warming, Stale, Down}`. `&self`, so a read lock suffices — but still
blocking, so it goes through `with_manager_read`.

Reads `last_handshake` via `self.wgapis.get(&entry.ifname)?.read_interface_data()`. Two platform
constraints found during design:

- **Windows `read_interface_data` requires the same `WGApi` that created the interface** (`self.adapter`),
  so this must go through `ServiceLocationManager::wgapis`; a missing map entry means `Down` → rebuild.
- **Windows `configure_peer` is a no-op** (it only logs). PSK rotation must therefore use
  `configure_interface(&full_config)` on **both** platforms — which is also better: an **in-place**
  rotation keeps routes/DNS/listen-port and the outage window is ~0. Document this so nobody "optimizes"
  it back to `configure_peer`.
- Treat `Some(UNIX_EPOCH)` as "never handshaked" — the same sentinel the stats task already special-cases.

**Three independent guards against re-authorizing a merely-offline tunnel:**

1. **Authorize-before-mutate** — the POST is first; failure leaves the interface untouched.
2. **`rx_bytes` liveness override** — if `rx_bytes` grew since the last pass the peer is demonstrably
   alive and only the timestamp is stale → downgrade to `Healthy`.
3. **Keepalive floor of 25s** whenever `posture_check_required`, so WireGuard's `REKEY_AFTER_TIME` (120s)
   refreshes `last_handshake` well inside the 180s window.

**Outcome handling per authorization attempt** (D4, D19, D20):

| Outcome | Action |
|---|---|
| `Ok(psk)` non-empty | apply in place |
| `Ok("")` | **D19** — location no longer has postures. Connect **without** a PSK, clear the in-memory flag; the next `SaveServiceLocations` fixes the JSON |
| `Unavailable` | record `reauth_pending_since`, **leave the interface untouched**, retry next tick |
| `Rejected` (403) | **D20** — tear the interface down. Keep retrying at 30s so remediation self-heals |
| `InvalidRequest` | treat as `Rejected` for teardown, but log distinctly (config drift, not posture failure) |

Why 180s < 300s: the client heals *before* core revokes, so the steady state is proactive
re-authorization rather than scrambling after revocation. **Document that this assumes the default
`peer_disconnect_threshold` of 300s** (D21).

### Phase 9 — core: activity events (D15)

`handle_posture_check` currently takes **only** the request — `device_info` is discarded at
`defguard_proxy_manager/src/handler.rs:1050-1051`. Thread it through (every other arm already does), then
emit the **existing** `PostureCheckPassed` / `PostureCheckFailed` variants at the two decision points,
building context via `parse_client_ip_agent` exactly as the MFA path does at `client_mfa.rs:242-244`. No
new event types, no migration.

Suppress duplicates: with D4's 30s retry a permanently-failing device would emit 2 events/minute forever.
Emit at most one `PostureCheckFailed` per `(location, device)` until the next success, using
`last_rejection` on the client side (core cannot distinguish a retry from a fresh attempt).

### Phase 10 — web: expose the combination (D8)

- `AddLocationPage.tsx:90-96` — drop `hidden: locationType === 'service'`
- `EditLocationPage.tsx:948-996` — remove the warning banner and `disabled={isServiceLocation}`
- `PostureChecksPage/postureChecks.ts:367-375` — drop the service-location filter (2 call sites)
- `messages/en/location.json:101` — delete the "can't be enabled" string; add an informational one
  explaining that the system service evaluates it at boot and non-compliant devices won't connect
- Note pre-logon + posture is Windows-only (D12)
- **Add `.min(1)` to the `keepalive_interval` zod schemas** (`AddLocationNetworkStep.tsx:15`,
  `EditLocationPage.tsx:155`) — they currently allow `0`, which breaks D6

`create_network` already accepts `posture_checks` and the Add wizard already sends it, so no API change
there; Edit uses the dedicated postures endpoint, which is fine.

### Phase 11 — docs + release notes

- Module docs: the JSON schema, the 30/60/180s constants and *why* they relate as they do, and the
  authorize-before-mutate invariant.
- **Required release note:** on Linux, `disk_encryption` is now evaluated against `/` by the system
  service rather than the partition holding the per-user client DB. A machine with encrypted `$HOME` on
  an unencrypted `/` (or vice versa) may see its posture result flip.
- Release note: pre-logon + posture is Windows-only.

---

## 6. Where the shared authorize function lives (after D10)

| Crate | Holds | Depends on |
|---|---|---|
| `common` | `VERSION`, `get_interface_name`, **`http::{post_with_headers, construct_platform_header}`** | reqwest, os_info, client-proto |
| `enterprise/posture-inspector` **(new)** | signal gathering; `device_posture_data(&Path)` | common, client-proto, sysinfo/wmi |
| `enterprise/posture` **(core-free)** | `PostureError`; `post_posture_check(...)`; **`authorize_posture_session(proxy_url, device_pubkey, location_id)`** | common, posture-inspector, reqwest |
| `core` | DB, `DAEMON_CLIENT`, **`posture_ipc::{get_posture_data, authorize_posture_session}`** | posture, posture-inspector, sqlx |

- **Daemon** calls `defguard_client_posture::authorize_posture_session` directly, with all three inputs
  from its own JSON. **Zero DB access** — that is what D1 buys.
- **App/CLI** do the DB lookups themselves (D16) then call `core::posture_ipc::*`, which is one gRPC hop
  to the daemon on Windows+Linux and in-process on macOS.
- **MFA path** still needs raw signals → `get_posture_data()` → `GetPostureData` RPC (now on Linux too).

The network call exists **once** (`post_posture_check`); the gather+call wrapper exists **once**
(`authorize_posture_session`), and both the daemon's RPC handler and its reconciler call that one.

---

## 7. Risks

### Must fix as part of this work

1. **RwLock guard across blocking syscalls.** Adding a 5s POST under the manager's `std::sync` write
   guard **will not compile** (`!Send` across `await`), and a `block_on` workaround deadlocks. Fixed by
   P5's `spawn_blocking` helpers + authorize-before-mutate. Also fixes lock poisoning aborting the daemon.
2. **gRPC inside a SQLite write transaction.** `sync_service_locations` is called with an open tx and
   would now fan out to N × (≤5s POST + rebuild), blocking all writers. Fixed by P4. Also fixes: a failed
   push currently rolls back the DB, so the next poll sees "unchanged" and **never retries**.
3. **`Key::decode` panics on a malformed PSK** (`client-proto/src/conversions.rs:77-79`) in a daemon built
   `panic = "abort"`. We are adding a second PSK source; harden it.
4. **Two incompatible `ServiceLocationMode` enums** — mitigated by the single conversion function, the
   discriminant-pinning test, and the mode-0 `warn!`. **Renumbering is not an option**: shipped 1.6/2.0
   clients persist the integer on disk.

### Design risks

5. **Same-device contention** — real and reachable via `find_by_id` in both the app and the CLI `--id`
   path; closed in P6.
6. **Boot-window posture failures** — Windows `wscsvc` is Automatic-Delayed-Start, so AV detection can be
   empty for the first 1–3 minutes of every boot → `CheckUnavailable` → 403. D4's retry self-heals;
   suppress alarm-level logs for the first ~2 min of daemon uptime.
7. **`keepalive_interval == 0` ⇒ perpetual re-authorization.** WireGuard only rekeys when the initiator
   sends, so `last_handshake` never advances on an idle tunnel and D6 fires forever. The web forms allow
   `0`. Mitigated by the 25s floor, the `rx_bytes` override, and the P10 zod change.
8. **Non-transactional authorization.** The proxy's 5s core timeout can 500 *after* core created the
   session, leaving an authorized session whose PSK we never received and the previous one already
   superseded. The health check is the recovery path.
9. **Listen-port churn.** `find_free_tcp_port()` per setup would change the source port on every rebuild,
   invalidating stateful NAT bindings; reuse the stored `listen_port`.
10. **Event/gateway flood.** Each re-auth ⇒ deauthorize + authorize commands and activity rows. Mitigated
    by never re-authorizing on transport failure, the 180s threshold, and the P9 suppression.
11. **Secret hygiene.** PSK stays in memory only; `skip_debug` on the response message.
12. **Proxy rate limiting** is off by default but per-IP when enabled — a NAT'd fleet retrying at 30s
    shares one bucket → 429, which we classify as `Unavailable` (correct: no teardown).
13. **Licence asymmetry** (D18, accepted). See §9.
14. **Two client definitions of "service location"** — `find_by_instance_id(.., false)` filters on
    `service_location_mode <= 1` while `is_service_location()` also requires MFA disabled. Combined with
    P1's third hole this is a latent invisible-location bug.

---

## 8. Verification

**Core setup.** Both licences active. Posture check requiring disk encryption, `min client 2.1.0`,
`allow_prerelease_client = true` for dev builds. Location `SL-AlwaysOn` (`ALWAYSON` + MFA disabled +
`keepalive_interval = 25`, **not 0**) and `SL-PreLogon` (Windows). Assigning the posture is itself the
**P1 acceptance test** — it returns 400 before P1. Then confirm the gateway withholds the peer until a
session exists.

**Linux (AlwaysOn).** Verify the JSON contains `proxy_url`, `device_pubkey`, per-location `network_id` and
`posture_check_required: true`, still `0600`/`0700`. Restart the service **with no user logged in**;
expect authorize → PSK → `wgN` up, and **`sudo wg show` must print `preshared key: (hidden)`** — if it
does not, the PSK was never applied. Confirm `dg list` hides the location and `dg connect --id <it>` is
rejected. Confirm the daemon links no sqlx.

**Windows (PreLogon).** Sign out fully, reboot, confirm from another machine that the session and gateway
peer exist at the logon screen. Sign in → PreLogon torn down; sign out → re-authorized with a **new**
session and the old one `Superseded`. Sleep/resume → `PowerEvent` wakes the reconciler.

**Force staleness (D6).** Easiest deterministic knob: on the gateway,
`wg set <if> peer <device_pubkey> preshared-key <random>` — handshakes fail immediately. Expect: within
≤60s of crossing 180s, `Stale` → authorize → `configure_interface` → fresh handshake, with the **interface
index unchanged** (in-place rotation). Core shows a new session, old one `Superseded`.

**Force rejection (D11, D20).** Set the policy's minimum client version to `99.0.0`. On a *fresh* connect
expect 403 with the reason string, **no interface created**, retry at exactly 30s forever, and
`DevicePostureCheckFailed` in the activity log **once**. Then repeat against an **already-up** tunnel and
confirm **D20**: the interface is torn down rather than left blackholing. Revert and confirm it heals
within 30s with no restart.

**Unassign postures from a live service location (D19).** With `SL-AlwaysOn` up and **no user logged in**,
remove all postures from it in core. Expect: the next authorize returns an empty PSK, the daemon
reconnects **without** a PSK, traffic resumes, and **no** `vpn_client_session` row is created.

**Must-not-regress.**

- Stop core → daemon gets `Unavailable` → **an existing healthy tunnel stays up** (the D5 guard).
- **D7:** change only `dns` → only that interface rebuilds. Change nothing → **no** interface is touched
  (compare interface indices and `wg show latest-handshake` before/after).
- **Upgrade in place** 2.0→2.1 with a healthy non-posture service location up → the diff resolves to
  `Unchanged` and **the tunnel must not flap**.
- **Downgrade** 2.1→2.0 → the old daemon still parses the new JSON (serde ignores unknown fields).
- Regular posture location from the GUI on Linux (PSK now via IPC) still shows `PostureCheckFailed`, not
  `Other`, on rejection. MFA+posture still works.
- `cargo fmt --all`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`;
  `cargo nextest run --workspace`; core's suite for P1–P2.

---

## 9. Accepted follow-ups (deliberately out of scope)

- **Linux resume hook.** Windows gets `PowerEvent(ResumeSuspend)`; Linux falls back to the 30s tick, so a
  tunnel can take up to 30s to repair after resume. A `systemd` `sleep.target` unit poking the daemon is
  the natural follow-up.
- **`peer_disconnect_threshold` awareness** (D21). If deployments start lowering it below 180s, plumb it
  through `ServiceLocation` and derive `stale_threshold = min(180s, threshold * 0.6)`.
- **Licence asymmetry** (D18). A lapsed `DevicePosture` licence silently stops enforcing posture on a
  still-running always-on tunnel. Worth raising with the product owner.
- **Two client definitions of "service location"** (risk 14) — worth unifying once P1's third hole is
  closed.

---

## 10. Still undecided

Two process questions were raised but not settled:

1. **Landing strategy.** Whether to land the three feature-independent refactors — the inspector crate
   split (D10), the DB-free `authorize_posture_session` (D16), and change-scoped reset (D7) — as separate
   prep PRs before the feature, or as one PR per repo. Each refactor is a strict improvement on its own
   and none needs the proto change, which argues for prep PRs.
2. **Which pre-existing bugs to fix in scope.** Risks 1–4 above are pre-existing but this feature makes 1
   and 2 materially worse (a network round-trip inside a lock held across blocking syscalls; a gRPC call
   fanning out inside an open write transaction). Risk 1 is arguably a prerequisite rather than a
   nice-to-have, since the code will not compile otherwise.
