# Video Tutorials

The video tutorials module fetches versioned JSON from the update service and uses
it to power two separate UI surfaces:

- route-bound tutorials inside the authenticated app shell
- placement-specific help content such as the migration wizard sidebar block

The JSON is fetched from `https://pkgs.defguard.net/api/content/video-tutorials`
by default via the shared update-service client.

## Current surfaces

- `NavTutorialsButton` opens the tutorials modal when the resolved app version
  contains at least one section video.
- `VideoSupportWidget` shows route-matched tutorial cards in the authenticated
  app shell.
- `MigrationWizardVideoGuide` shows a placement-driven video thumbnail and docs
  link in the migration wizard sidebar when the resolved app version contains a
  `placements.migrationWizard` entry.

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
              "docsUrl": "https://docs.defguard.net/introduction"
            }
          ]
        }
      ],
      "placements": {
        "migrationWizard": {
          "youtubeVideoId": "xyz987GHI12",
          "title": "Migration wizard guide",
          "description": "How to migrate your deployment.",
          "docsUrl": "https://docs.defguard.net/migration"
        }
      }
    }
  }
}
```

## Contract rules

- `versions` is required.
- Each version key must be `major.minor` or `major.minor.patch`.
- Each version entry must contain `sections`.
- `sections` may be an empty array.
- `placements` is optional.
- `placements.migrationWizard` is optional.
- Route tutorial videos require `appRoute`.
- Placement videos do not use `appRoute`.

## Version resolution

The resolver selects the single newest version whose key is less than or equal
to the running app version.

- Route tutorial sections come from that selected version entry's `sections`.
- Placement content comes from that same selected version entry's `placements`.
- There is no fallback to older versions once a newer eligible version is selected.

Example:

- app version `2.3.0`
- JSON has `2.1` and `2.2`

Result:

- `2.2` is selected
- both route sections and migration-wizard placement come only from `2.2`

If `2.2` omits `placements.migrationWizard`, the migration wizard shows nothing,
even if `2.1` defined it.

## Local testing

You can serve local JSON from `web/public/content/video-tutorials` or override
`VITE_VIDEO_TUTORIALS_URL` / `VITE_UPDATE_BASE_URL` in `web/.env.local`.

The default fetch path remains:

```text
https://pkgs.defguard.net/api/content/video-tutorials
```
