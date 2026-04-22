# Video Tutorials

The video tutorials module displays YouTube-based help content sourced from the
update service. It now powers two kinds of UI:

- route-bound tutorials inside the authenticated app shell
- placement-specific help content in wizard sidebars

The configuration is fetched as static JSON from
`https://pkgs.defguard.net/api/content/video-tutorials` by default.

Route-based tutorials are mounted in `src/routes/_authorized/_default.tsx` and
remain available across the authenticated layout. Placement-based content is
rendered inside wizard step layouts under `/_wizard/migration` and
`/_wizard/setup`.

A launcher button (`NavTutorialsButton`) is shown in the navigation only when
the resolved app version contains at least one tutorial section with videos.
Clicking it opens a two-panel modal: the left panel shows a searchable,
collapsible list of all tutorial sections; the right panel shows the selected
video in an embedded YouTube player with its title, description, and links to
the relevant app page and documentation.

A separate floating "Video support" button appears in the bottom-right corner
when at least one route-matched video is available for the current page.
Clicking it opens a floating list of video cards with thumbnails and titles.
Clicking on a specific card opens a modal with an embedded YouTube player.

Wizards use the same JSON source, but read a dedicated placement entry from the
resolved version and render a thumbnail plus documentation card in the sidebar.
Current placement keys are `migrationWizard`, `initialSetupWizard`, and
`autoAdoptionWizard`.

While a video is loading a skeleton placeholder is shown; if the video fails to
load within 8 seconds, a "Video unavailable" message is displayed instead with
a clickable YouTube link.

---

## Module structure

```text
video-tutorials/
├── types.ts                     — shared tutorial + placement types
├── data.ts                      — Zod schema, parseVideoTutorials(), videoTutorialsPath
├── resolver.ts                  — version selection and placement helpers
├── resolved.tsx                 — hooks for resolved sections, placements, and route matches
├── route-key.ts                 — canonicalizeRouteKey(), getNavRoot()
├── route-label.ts               — getRouteLabel() — maps known routes to translated nav label strings
├── store.ts                     — useVideoTutorialsModal Zustand store (isOpen)
├── version.ts                   — parseVersion(), compareVersions()
├── VideoTutorialsModal.tsx      — modal shell (open/close state only)
├── VideoSupportWidget.tsx       — floating widget (launcher button + route-contextual overlay)
├── style.scss
└── components/
    ├── modal/
    │   ├── ModalContent/        — modal inner content (player, video info, links, section list)
    │   └── VideoList/           — searchable, collapsible section list panel
    ├── widget/
    │   ├── NavTutorialsButton/  — nav sidebar button (opens modal)
    │   ├── Thumbnail/           — video thumbnail image
    │   ├── VideoCard/           — clickable video card shown in the widget list
    │   └── VideoOverlay/        — floating overlay player shown on the current route or wizard
    └── VideoPlayer/             — shared iframe player used by both overlay and modal
```

---

## Testing without remote API access

In production the module fetches its mapping via the shared update-service axios
client. The default URL is:

```text
https://pkgs.defguard.net/api/content/video-tutorials
```

This is composed from the client's base URL (`https://pkgs.defguard.net/api`,
overridable via `VITE_UPDATE_BASE_URL`) and the default path
(`/content/video-tutorials`, overridable via `VITE_VIDEO_TUTORIALS_URL`).

Both variables are read at build time and live in `web/.env.local` (git-ignored).

### Serve a local file via the Vite dev server (recommended)

The simplest approach: place your test JSON at `web/public/content/video-tutorials`
(no file extension) or `web/public/content/video-tutorials.json`. The Vite dev
server serves everything in `web/public/` at the root, so the file is immediately
available on the same origin, with no extra process or CORS issues.

1. Create the directory and file:

   ```bash
   mkdir -p web/public/content
   # create web/public/content/video-tutorials with the JSON structure shown below
   ```

2. If you used no file extension, no env override is needed because the default
   path already matches. If you used `.json`, set:

   ```text
   # web/.env.local
   VITE_VIDEO_TUTORIALS_URL=/content/video-tutorials.json
   ```

3. Start the dev server as usual (`pnpm dev`). The module will pick up the file
   on page load.

> Do not commit the test file. `web/public/content/` is not git-ignored by
> default, so add it to your local `.git/info/exclude` or a global gitignore
> if you want to keep it permanently.

### Redirect the entire update service to a separate local server

Use this approach when you need to test the client artifact check alongside the
video tutorials fetch, or when you want to simulate a real remote server:

```text
# web/.env.local
VITE_UPDATE_BASE_URL=http://localhost:4000
```

Then serve a JSON file at `http://localhost:4000/content/video-tutorials`.

> The local server must respond with appropriate `Access-Control-Allow-Origin`
> CORS headers since it runs on a different origin than the Vite dev server.

### Override only the video tutorials path

To redirect just the video tutorials fetch without affecting other update-service
calls, override the path only:

```text
# web/.env.local
VITE_VIDEO_TUTORIALS_URL=/content/my-test-config
```

The path is still resolved against `VITE_UPDATE_BASE_URL` (or the default
`https://pkgs.defguard.net/api`), so this is most useful when you have a test
endpoint on the same server.

---

## JSON structure

```jsonc
{
  "versions": {
    "2.2": {
      "sections": [
        {
          "name": "Getting Started",
          "videos": [
            {
              "youtubeVideoId": "abc123DEF45",
              "title": "Defguard overview",
              "description": "A high-level walkthrough of Defguard.",
              "appRoute": "/vpn-overview",
              "contextAppRoutes": [
                "/vpn-overview/$locationId",
                "/settings/"
              ],
              "docsUrl": "https://docs.defguard.net/introduction"
            }
          ]
        }
      ],
        "placements": {
          "migrationWizard": {
            "default": {
              "video": {
                "youtubeVideoId": "xyz987GHI12",
                "title": "Migration wizard guide"
              },
              "docs": [
                {
                  "docsTitle": "Defguard Configuration Guide",
                  "docsUrl": "https://docs.defguard.net/migration"
                }
              ]
            },
            "steps": {
              "ca": {
                "video": {
                  "youtubeVideoId": "aaaBBBccc11",
                  "title": "Certificate authority guide"
                },
                "docs": [
                  {
                    "docsTitle": "Certificate authority documentation",
                    "docsUrl": "https://docs.defguard.net/migration/ca"
                  },
                  {
                    "docsTitle": "Certificate authority troubleshooting",
                    "docsUrl": "https://docs.defguard.net/migration/ca/troubleshooting"
                  }
                ]
              }
            }
          },
          "initialSetupWizard": {
            "default": {
              "video": {
                "youtubeVideoId": "bbbCCCddd22",
                "title": "Initial setup guide"
              },
              "docs": [
                {
                  "docsTitle": "Initial setup documentation",
                  "docsUrl": "https://docs.defguard.net/setup"
                }
              ]
            },
            "steps": {
              "adminUser": {
                "docs": [
                  {
                    "docsTitle": "Admin user documentation",
                    "docsUrl": "https://docs.defguard.net/setup/admin-user"
                  }
                ]
              }
            }
          },
          "autoAdoptionWizard": {
            "default": {
              "docs": [
                {
                  "docsTitle": "Auto adoption documentation",
                  "docsUrl": "https://docs.defguard.net/auto-adoption"
                }
              ]
            },
            "steps": {
              "vpnSettings": {
                "video": {
                  "youtubeVideoId": "eeeFFFggg55",
                  "title": "VPN settings guide"
                }
              }
            }
          }
        }
      }
   }
}
```

### Field reference

Route tutorial video fields:

| Field | Required | Description |
|---|---|---|
| `youtubeVideoId` | Yes | Exactly 11 characters: letters, digits, `-`, `_`. Found in any YouTube video URL after `?v=` or in the short URL path. |
| `title` | Yes | Non-empty string. Displayed in the section list and as the heading above the player. |
| `description` | Yes | Non-empty string. Displayed as body text below the player in the tutorials modal. |
| `appRoute` | Yes | Must start with `/`. Use TanStack Router route definition paths (e.g. `/vpn-overview`, `/vpn-overview/$locationId`), not runtime URLs with concrete param values. |
| `contextAppRoutes` | No | Optional non-empty array of additional in-app route definition paths where the tutorial should also appear. Each entry must start with `/`. |
| `docsUrl` | No | Optional valid URL. When present, shown as the external documentation link in the tutorials modal. |

Wizard placement fields:

| Field | Required | Description |
|---|---|---|
| `video` | No | Optional video block shown in the wizard sidebar. |
| `docs` | No | Optional non-empty array of documentation links shown one under another in the wizard sidebar. |

`video` fields:

| Field | Required | Description |
|---|---|---|
| `youtubeVideoId` | Yes | Used to render the thumbnail and embedded player in the wizard sidebar. |
| `title` | Yes | Displayed next to the thumbnail and used as the iframe title. |

`docs` item fields:

| Field | Required | Description |
|---|---|---|
| `docsTitle` | Yes | Text shown in the wizard documentation card. |
| `docsUrl` | Yes | External URL opened from the wizard documentation card. |

Wizard placement structure:

- `default`: optional fallback guide used when the current step has no dedicated entry
- `steps`: optional map of step key to guide data

Each placement object must define at least one of `video` or `docs`.
If `docs` is present, it must contain at least one item.

`placements` is a string-keyed record, so placement keys are not hardcoded in the
schema. The application currently uses these keys:

- `migrationWizard`
- `initialSetupWizard`
- `autoAdoptionWizard`

The keys in each placement's `steps` map should match frontend step IDs.
Examples:

- Migration: `general`, `ca`, `caSummary`, `edgeDeployment`, `edge`, `edgeAdoption`, `internalUrlSettings`, `internalUrlSslConfig`, `externalUrlSettings`, `externalUrlSslConfig`, `confirmation`, `welcome`
- Initial setup: `adminUser`, `generalConfig`, `certificateAuthority`, `certificateAuthoritySummary`, `edgeDeploy`, `edgeComponent`, `edgeAdoption`, `internalUrlSettings`, `internalUrlSslConfig`, `externalUrlSettings`, `externalUrlSslConfig`, `confirmation`
- Auto adoption: `adminUser`, `internalUrlSettings`, `internalUrlSslConfig`, `externalUrlSettings`, `externalUrlSslConfig`, `vpnSettings`, `mfaSetup`, `summary`

### Section structure

Each version value is an object with:

- `sections`: ordered route-based tutorial sections
- `placements`: optional surface-specific content entries

`sections` is required for every version entry, but it may be an empty array.

Sections are displayed in the order they appear in the array; videos within a
section are displayed in their array order.

There is no concept of a per-section `appRoute`. Route association is set per
video via the `appRoute` field. A section can contain videos for multiple routes.

---

## Version resolution

`resolveVersion()` selects the single newest version whose key is less than or
equal to the runtime app version.

Rules:

- Only version keys that are `<=` the runtime app version are eligible.
- The single newest eligible version is selected.
- If the runtime app version has a prerelease or build suffix (e.g. `2.2.0-rc.1`)
  the suffix is stripped before matching, so `2.2.0-rc.1` resolves as `2.2.0`.
- If the app version string cannot be parsed, or no eligible version exists,
  resolution returns `null` / `[]` depending on the helper.

Consumers built on top of that selected version:

- `resolveSections()` returns `selectedVersion.sections`
- `resolveVideoGuidePlacement()` returns a step-aware placement from the selected version

There is no fallback to older versions once a newer eligible version has been
selected. If `2.2` is selected and omits a placement key, that wizard shows
nothing even if `2.1` defined that placement.

Within the selected version, wizard guide resolution uses this fallback order:

1. `placements[placementKey].steps[currentStep]`
2. `placements[placementKey].default`
3. `null`

Fallback to `default` only happens when the whole step entry is missing.
If a step entry exists with only `docs` or only `video`, it is used as-is and is
not merged with `default`.

---

## appRoute matching

Route matching is a plain string equality check after canonicalization
(`canonicalizeRouteKey` in `route-key.ts`). The current route key comes from
TanStack Router's `fullPath` for the active content match. This is always the
route definition string, never an instantiated URL with real param values.

For example, when the user is on `/vpn-overview/42`, TanStack Router reports
`fullPath` as `/vpn-overview/$locationId` (the template). Canonicalization trims
whitespace, ensures a leading `/`, and strips a trailing `/`. The same
canonicalization is applied to every `video.appRoute` and `video.contextAppRoutes`
value before comparison.

A route tutorial is shown when the current route matches either:

- `appRoute`
- any entry in `contextAppRoutes`

### Parameterized routes

A video with `appRoute: "/vpn-overview"` and
`contextAppRoutes: ["/vpn-overview/$locationId"]` matches both the overview page
and any location detail page, regardless of the concrete `locationId` in the
URL. Tutorials are associated with route shapes, not specific records.

Do not use runtime URLs as `appRoute`. A value like `/vpn-overview/42` will
never match because `fullPath` always contains the template placeholder
`$locationId`, not the concrete value.

### The "Go to" link in the modal

The tutorials modal shows a "Go to [Page]" link for the selected route tutorial.
Because a route definition string with `$param` placeholders is not a valid
navigation target, the link target is derived by stripping everything from the
first dynamic segment onward (`getNavRoot` in `route-key.ts`).

| `appRoute`                  | "Go to" navigates to |
| --------------------------- | --------------------- |
| `/vpn-overview`             | `/vpn-overview`       |
| `/vpn-overview/$locationId` | `/vpn-overview`       |
| `/acl/rules/$ruleId/edit`   | `/acl/rules`          |

The translated navigation label for the resulting parent route is used as the
link text (looked up via `getRouteLabel` in `route-label.ts`). If the parent
route has no entry in that map, the raw path is shown as a fallback.
