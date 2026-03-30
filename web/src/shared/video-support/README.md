# Video Support

The video support widget displays route-specific YouTube video tutorials inside
the authenticated app shell. A floating "Video support" button appears in the
bottom-right corner when videos are available for the current route. Clicking
it opens a panel listing the relevant videos; clicking a video opens a full-screen
overlay with an embedded YouTube player.

The widget is mounted in `src/routes/_authorized/_default.tsx` and is therefore
available across the entire authenticated layout.

---

## Testing without remote API access

In production the widget fetches its video mapping via the shared update-service
axios client. The default URL is:

```
https://pkgs.defguard.net/api/content/video-support
```

This is composed from the client's base URL (`https://pkgs.defguard.net/api`,
overridable via `VITE_UPDATE_BASE_URL`) and the default path
(`/content/video-support`, overridable via `VITE_VIDEO_SUPPORT_URL`).

Both variables are read at build time and live in `web/.env.local` (git-ignored).

### Redirect the entire update service to a local server

The simplest approach for local development — redirects both the video support
config and the client artifact checks to your local server:

```
# web/.env.local
VITE_UPDATE_BASE_URL=http://localhost:4000
```

Then serve a JSON file at `http://localhost:4000/content/video-support` (see
[JSON structure](#json-structure) below for the expected format):

```bash
# example using Python's built-in server from the directory containing your file
python3 -m http.server 4000
```

> The local server must respond with appropriate `Access-Control-Allow-Origin`
> CORS headers if it runs on a different origin than the Vite dev server.

### Override only the video support path

To redirect just the video support fetch without affecting other update-service
calls, override the path only:

```
# web/.env.local
VITE_VIDEO_SUPPORT_URL=/content/my-test-config
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
    "2.2": {
      // Route key: must start with "/". Use route templates, not runtime URLs.
      // Trailing slashes are stripped (except for the root "/").
      "/settings": [
        {
          // Required. Exactly 11-character YouTube video ID (from the video URL).
          "youtubeVideoId": "abc123DEF45",

          // Required. Non-empty display title shown on the card.
          "title": "Configuring settings"
        }
      ],

      // A route can have multiple videos.
      "/users": [
        { "youtubeVideoId": "xyz987GHI12", "title": "Managing users" },
        { "youtubeVideoId": "lmn456JKL78", "title": "User roles overview" }
      ],

      // Dynamic route segments use TanStack Router template syntax.
      "/vpn-overview/$locationId": [
        { "youtubeVideoId": "pqr321MNO65", "title": "VPN location walkthrough" }
      ],

      // An explicit empty array suppresses fallback to older versions for this
      // route — the widget will not appear on this route for version 2.2 users.
      "/legacy-page": []
    },

    // Older version. Only consulted when the current app version is older than
    // 2.2, or when 2.2 does not define the route being looked up.
    "2.0": {
      "/settings": [
        { "youtubeVideoId": "old000SET00", "title": "Settings (legacy)" }
      ]
    }
  }
}
```

### Field reference

| Field | Required | Description |
|---|---|---|
| `youtubeVideoId` | Yes | Exactly 11 characters: letters, digits, `-`, `_`. Found in any YouTube video URL after `?v=` or in the short URL path. |
| `title` | Yes | Non-empty string. Displayed on the video card. |

### Route key rules

- Must start with `/`.
- Use **route template paths** (e.g. `/vpn-overview/$locationId`), not runtime
  URLs with actual parameter values. The widget matches against TanStack Router's
  `fullPath` template, not the resolved pathname.
- Trailing slashes are stripped during parsing, so `/settings/` and `/settings`
  are treated as the same key. Duplicates after normalisation are rejected.

---

## Version resolution

When looking up videos for the current route, the resolver iterates mapped
versions from **newest to oldest** and returns the video list from the first
version that defines the current route key.

- If the runtime app version has a prerelease or build suffix (e.g. `2.2.0-rc.1`)
  the suffix is stripped before matching, so `2.2.0-rc.1` resolves as `2.2.0`.
- If a version defines a route with an **explicit empty array**, that empty result
  is returned and older versions are **not** consulted — this is intentional and
  allows suppressing the widget on a specific route for a given version.
- If no version defines the current route, the widget does not appear.
