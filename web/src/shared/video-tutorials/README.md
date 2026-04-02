# Video Tutorials

The video tutorials module displays YouTube video tutorials inside the
authenticated app shell.

The configuration for the module (list of videos for specific app version) is fetched
as static JSON from 'https://pkgs.defguard.net/api/content/video-tutorials.

A launcher button (`NavTutorialsButton`) is shown in the
navigation bar if the JSON config is successfully fetched and parsed.
Clicking it opens a two-panel modal: the left panel shows
a searchable, collapsible list of all tutorial sections; the right panel shows
the selected video in an embedded YouTube player with its title, description,
and links to the relevant app page and documentation.

A separate floating "Video support" button appears in the
bottom-right corner when at least one video is available for the current route.
Clicking it opens a floating list of video cards with thumbnails and titles.
Clicking on a specific card opens a modal with an embedded YouTube player.

The module is mounted in `src/routes/_authorized/_default.tsx` and is therefore
available across the entire authenticated layout.

While a video is loading a
skeleton placeholder is shown; if the video fails to load within 8 seconds, a
"Video unavailable" message is displayed instead with a clickable YouTube link.


---

## Module structure

```
video-tutorials/
├── types.ts                     — VideoTutorial, VideoTutorialsSection, VideoTutorialsMappings types
├── data.ts                      — Zod schema, parseVideoTutorials(), videoTutorialsPath
├── resolver.ts                  — resolveVideoTutorials(), resolveAllSections()
├── resolved.tsx                 — useResolvedVideoTutorials, useAllVideoTutorialsSections, useVideoTutorialsRouteKey
├── route-key.ts                 — canonicalizeRouteKey(), getNavRoot()
├── route-label.ts               — getRouteLabel() — maps known routes to translated nav label strings
├── store.ts                     — useVideoTutorialsModal Zustand store (isOpen)
├── version.ts                   — parseVersion(), compareVersions()
├── VideoTutorialsModal.tsx      — modal shell (open/close state only)
├── VideoTutorialsModal.scss
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
    │   └── VideoOverlay/        — floating overlay player shown on the current route
    └── VideoPlayer/             — shared iframe player used by both overlay and modal
```

---

## Testing without remote API access

In production the module fetches its video mapping via the shared update-service
axios client. The default URL is:

```
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
available on the same origin — no extra process, no CORS issues.

1. Create the directory and file:

   ```bash
   mkdir -p web/public/content
   # create web/public/content/video-tutorials with the JSON structure shown below
   ```

2. If you used no file extension, no env override is needed — the default path
   `/content/video-tutorials` already matches. If you used `.json`, set:

   ```
   # web/.env.local
   VITE_VIDEO_TUTORIALS_URL=/content/video-tutorials.json
   ```

3. Start the dev server as usual (`pnpm dev`). The module will pick up the file
   on page load.

> Do not commit the test file — `web/public/content/` is not git-ignored by
> default, so add it to your local `.git/info/exclude` or a global gitignore
> if you want to keep it permanently.

### Redirect the entire update service to a separate local server

Use this approach when you need to test the client artifact check alongside the
video tutorials fetch, or when you want to simulate a real remote server:

```
# web/.env.local
VITE_UPDATE_BASE_URL=http://localhost:4000
```

Then serve a JSON file at `http://localhost:4000/content/video-tutorials`:

```bash
# example using Python's built-in server from the directory containing your file
python3 -m http.server 4000
```

> The local server must respond with appropriate `Access-Control-Allow-Origin`
> CORS headers since it runs on a different origin than the Vite dev server.

### Override only the video tutorials path

To redirect just the video tutorials fetch without affecting other update-service
calls, override the path only:

```
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
    // Version key: "major.minor" or "major.minor.patch"
    "2.2": [
      // Each entry is a named section shown as a collapsible group in the modal.
      {
        "name": "Getting Started",
        "videos": [
          {
            // Required. Exactly 11-character YouTube video ID (from ?v= or short URL).
            "youtubeVideoId": "abc123DEF45",

            // Required. Non-empty display title shown in the list and above the player.
            "title": "Defguard overview",

            // Required. Non-empty description shown below the player.
            "description": "A high-level walkthrough of Defguard.",

            // Required. In-app route this video is associated with.
            // Must start with "/". Use TanStack Router template paths.
            "appRoute": "/vpn-overview",

            // Required. External documentation URL shown as a link below the player.
            "docsUrl": "https://docs.defguard.net/introduction"
          },
          {
            "youtubeVideoId": "xyz987GHI12",
            "title": "Initial setup wizard",
            "description": "Configure your first location and gateway.",
            "appRoute": "/locations",
            "docsUrl": "https://docs.defguard.net/admin-and-features/initial-setup"
          }
        ]
      },
      {
        "name": "VPN & Firewall",
        "videos": [
          {
            "youtubeVideoId": "pqr321MNO65",
            "title": "Firewall rules explained",
            "description": "How ACL rules control traffic between VPN peers.",
            // Dynamic route segments use TanStack Router template syntax.
            "appRoute": "/acl/rules",
            "docsUrl": "https://docs.defguard.net/admin-and-features/firewall/rules"
          }
        ]
      }
    ],

    // Older version. Only consulted by resolveVideoTutorials() for the floating
    // launcher button when the current route has no match in a newer version.
    // resolveAllSections() (used by the modal) always uses the newest eligible version only.
    "2.0": [
      {
        "name": "Getting Started",
        "videos": [
          {
            "youtubeVideoId": "old000OVR00",
            "title": "Defguard overview (legacy)",
            "description": "Legacy overview video.",
            "appRoute": "/vpn-overview",
            "docsUrl": "https://docs.defguard.net/introduction"
          }
        ]
      }
    ]
  }
}
```

### Field reference

| Field | Required | Description |
|---|---|---|
| `youtubeVideoId` | Yes | Exactly 11 characters: letters, digits, `-`, `_`. Found in any YouTube video URL after `?v=` or in the short URL path. |
| `title` | Yes | Non-empty string. Displayed in the section list and as the heading above the player. |
| `description` | Yes | Non-empty string. Displayed as body text below the player. |
| `appRoute` | Yes | Must start with `/`. Use TanStack Router **route definition** paths (e.g. `/vpn-overview`, `/vpn-overview/$locationId`), not runtime URLs with concrete param values. Trailing slashes are stripped during canonicalization. See [appRoute matching](#approute-matching) for full details. |
| `docsUrl` | Yes | Valid URL. Shown as a "Learn more in Documentation" link below the player. |

### Section structure

Each version value is an **ordered array of sections**. Sections are displayed
in the order they appear in the array; videos within a section are displayed in
their array order.

There is no concept of a per-section `appRoute` — route association is set
per-video via the `appRoute` field. A section can contain videos for multiple
different routes.

---

## Version resolution

The module uses two different resolution strategies depending on the context:

### Modal (all sections)

`resolveAllSections()` returns the **complete section list** from the single
newest version whose key is ≤ the runtime app version. It does **not** fall back
to older versions — the modal always shows one version's worth of content.

### Floating launcher button (route-specific videos)

`resolveVideoTutorials()` walks eligible versions from **newest to oldest** and
returns the videos from the first version that has at least one video whose
`appRoute` matches the current route:

- Only version keys that are ≤ the runtime app version are eligible.
- Eligible versions are walked newest-to-oldest.
- The first version with a matching `appRoute` video wins; all matching videos
  from that version (across all sections) are returned.
- If no version has a matching video for the current route, the launcher button
  is not shown.

### Common rules (both strategies)

- If the runtime app version has a prerelease or build suffix (e.g. `2.2.0-rc.1`)
  the suffix is stripped before matching, so `2.2.0-rc.1` resolves as `2.2.0`.
- If the app version string cannot be parsed, or no eligible version exists,
  both functions return an empty result.

---

## appRoute matching

Route matching is a plain string equality check after canonicalization
(`canonicalizeRouteKey` in `route-key.ts`). The current route key comes from
TanStack Router's `fullPath` for the active content match — this is always the
**route definition string**, never an instantiated URL with real param values.

For example, when the user is on `/vpn-overview/42`, TanStack Router reports
`fullPath` as `/vpn-overview/$locationId` (the template). Canonicalization trims
whitespace, ensures a leading `/`, and strips a trailing `/`. The same
canonicalization is applied to every `video.appRoute` value before comparison.

### Parameterized routes

A video with `appRoute: "/vpn-overview/$locationId"` matches whenever the user
is on **any** location detail page, regardless of the concrete `locationId` in
the URL. This is by design: tutorials are associated with route shapes, not
specific data records.

**Do not use runtime URLs as `appRoute`.** A value like `/vpn-overview/42` will
never match because `fullPath` always contains the template placeholder
`$locationId`, not the concrete value.

### The "Go to" link in the modal

The modal shows a "Go to [Page]" link for the selected video. Because a route
definition string with `$param` placeholders is not a valid navigation target
(the param value to substitute is not known), the link target is derived by
stripping everything from the first dynamic segment onward (`getNavRoot` in
`route-key.ts`):

| `appRoute`                     | "Go to" navigates to |
| ------------------------------ | -------------------- |
| `/vpn-overview`                | `/vpn-overview`      |
| `/vpn-overview/$locationId`    | `/vpn-overview`      |
| `/acl/rules/$ruleId/edit`      | `/acl/rules`         |

The translated navigation label for the resulting parent route is used as the
link text (looked up via `getRouteLabel` in `route-label.ts`). If the parent
route has no entry in that map, the raw path is shown as a fallback.
