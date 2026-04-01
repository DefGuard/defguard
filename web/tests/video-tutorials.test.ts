import { describe, expect, it } from 'vitest';
import { parseVideoTutorials } from '../src/shared/video-tutorials/data';
import { resolveAllSections, resolveVideoTutorials } from '../src/shared/video-tutorials/resolver';
import { canonicalizeRouteKey } from '../src/shared/video-tutorials/route-key';
import { parseVersion } from '../src/shared/video-tutorials/version';
import type { VideoTutorialsMappings } from '../src/shared/video-tutorials/types';

// ---------------------------------------------------------------------------
// canonicalizeRouteKey
// ---------------------------------------------------------------------------

describe('canonicalizeRouteKey', () => {
  it('should preserve root slash', () => {
    expect(canonicalizeRouteKey('/')).toBe('/');
  });

  it('should add missing leading slash', () => {
    expect(canonicalizeRouteKey('users')).toBe('/users');
  });

  it('should strip non-root trailing slash', () => {
    expect(canonicalizeRouteKey('/settings/')).toBe('/settings');
  });

  it('should strip trailing slash and add leading slash together', () => {
    expect(canonicalizeRouteKey('settings/')).toBe('/settings');
  });

  it('should trim surrounding whitespace', () => {
    expect(canonicalizeRouteKey('  /users  ')).toBe('/users');
  });

  it('should preserve dynamic route templates', () => {
    expect(canonicalizeRouteKey('/vpn-overview/$locationId')).toBe('/vpn-overview/$locationId');
  });

  it('should not strip the root slash when input is only a slash', () => {
    expect(canonicalizeRouteKey('  /  ')).toBe('/');
  });
});

// ---------------------------------------------------------------------------
// parseVersion
// ---------------------------------------------------------------------------

describe('parseVersion', () => {
  it('should parse major.minor', () => {
    expect(parseVersion('2.2')).toEqual({ major: 2, minor: 2, patch: 0 });
  });

  it('should parse major.minor.patch', () => {
    expect(parseVersion('2.2.1')).toEqual({ major: 2, minor: 2, patch: 1 });
  });

  it('should strip prerelease suffix', () => {
    expect(parseVersion('2.2.0-beta')).toEqual({ major: 2, minor: 2, patch: 0 });
  });

  it('should strip build metadata', () => {
    expect(parseVersion('2.2.0+build.1')).toEqual({ major: 2, minor: 2, patch: 0 });
  });

  it('should strip prerelease and build metadata together', () => {
    expect(parseVersion('2.2.0-beta+build.1')).toEqual({ major: 2, minor: 2, patch: 0 });
  });

  it('should return null for empty string', () => {
    expect(parseVersion('')).toBeNull();
  });

  it('should return null for non-semver string', () => {
    expect(parseVersion('not-a-version')).toBeNull();
  });

  it('should return null for single number', () => {
    expect(parseVersion('2')).toBeNull();
  });

  it('should trim whitespace before parsing', () => {
    expect(parseVersion('  2.1  ')).toEqual({ major: 2, minor: 1, patch: 0 });
  });
});

// ---------------------------------------------------------------------------
// Shared fixture helpers (new schema)
// ---------------------------------------------------------------------------

const makeVideo = (id: string, appRoute: string) => ({
  youtubeVideoId: id,
  title: `Video ${id}`,
  description: `Description for ${id}`,
  appRoute,
  docsUrl: 'https://docs.defguard.net/test',
});

const makeMappings = (): VideoTutorialsMappings => ({
  '2.0': [
    {
      name: 'Identity',
      videos: [makeVideo('usrGuide200', '/users')],
    },
  ],
  '2.2': [
    {
      name: 'Identity',
      videos: [makeVideo('usrGuide220', '/users')],
    },
    {
      name: 'Admin',
      videos: [makeVideo('setGuide220', '/settings')],
    },
  ],
});

// ---------------------------------------------------------------------------
// resolveVideoTutorials
// ---------------------------------------------------------------------------

describe('resolveVideoTutorials', () => {
  it('should return videos for an exact version match', () => {
    const result = resolveVideoTutorials(makeMappings(), '2.2', '/users');
    expect(result).toHaveLength(1);
    expect(result[0].youtubeVideoId).toBe('usrGuide220');
  });

  it('should return the most recent eligible version when the exact version defines the route', () => {
    const result = resolveVideoTutorials(makeMappings(), '2.2', '/users');
    // 2.2 defines /users, so we get the 2.2 entry (not the older 2.0 entry)
    expect(result[0].youtubeVideoId).toBe('usrGuide220');
  });

  it('should fall back to 2.0 for a route only defined there when running 2.2', () => {
    const mappings: VideoTutorialsMappings = {
      '2.0': [{ name: 'Identity', videos: [makeVideo('usrGuide200', '/users')] }],
      '2.2': [{ name: 'Admin', videos: [makeVideo('setGuide220', '/settings')] }],
    };
    const result = resolveVideoTutorials(mappings, '2.2', '/users');
    expect(result[0].youtubeVideoId).toBe('usrGuide200');
  });

  it('should not use a version newer than the runtime version', () => {
    const result = resolveVideoTutorials(makeMappings(), '2.0', '/settings');
    expect(result).toHaveLength(0);
  });

  it('should return empty array when no version defines the route', () => {
    const result = resolveVideoTutorials(makeMappings(), '2.2', '/nonexistent');
    expect(result).toHaveLength(0);
  });

  it('should return empty array for an unparseable app version', () => {
    const result = resolveVideoTutorials(makeMappings(), '', '/users');
    expect(result).toHaveLength(0);
  });

  it('should strip prerelease from runtime version before resolving', () => {
    const result = resolveVideoTutorials(makeMappings(), '2.2.0-beta', '/users');
    expect(result[0].youtubeVideoId).toBe('usrGuide220');
  });

  it('should collect matching videos from multiple sections in the same version', () => {
    const mappings: VideoTutorialsMappings = {
      '2.2': [
        { name: 'Section A', videos: [makeVideo('videoA001', '/users')] },
        { name: 'Section B', videos: [makeVideo('videoB001', '/users')] },
      ],
    };
    const result = resolveVideoTutorials(mappings, '2.2', '/users');
    expect(result).toHaveLength(2);
    expect(result.map((v) => v.youtubeVideoId)).toContain('videoA001');
    expect(result.map((v) => v.youtubeVideoId)).toContain('videoB001');
  });

  it('should canonicalize appRoute trailing slash when matching', () => {
    // /locations/ in the JSON should match the /locations route key
    const mappings: VideoTutorialsMappings = {
      '2.2': [{ name: 'VPN', videos: [makeVideo('loc001xxxxx', '/locations/')] }],
    };
    const result = resolveVideoTutorials(mappings, '2.2', '/locations');
    expect(result).toHaveLength(1);
    expect(result[0].youtubeVideoId).toBe('loc001xxxxx');
  });
});

// ---------------------------------------------------------------------------
// resolveAllSections
// ---------------------------------------------------------------------------

describe('resolveAllSections', () => {
  it('should return all sections from the newest eligible version', () => {
    const result = resolveAllSections(makeMappings(), '2.2');
    expect(result).toHaveLength(2);
    expect(result[0].name).toBe('Identity');
    expect(result[1].name).toBe('Admin');
  });

  it('should respect the app version ceiling (not use versions newer than app)', () => {
    const result = resolveAllSections(makeMappings(), '2.0');
    expect(result).toHaveLength(1);
    expect(result[0].name).toBe('Identity');
  });

  it('should return empty array for an unparseable app version', () => {
    const result = resolveAllSections(makeMappings(), '');
    expect(result).toHaveLength(0);
  });

  it('should return empty array when no eligible version exists', () => {
    const result = resolveAllSections(makeMappings(), '1.0');
    expect(result).toHaveLength(0);
  });

  it('should pick the newest when multiple versions are eligible', () => {
    const result = resolveAllSections(makeMappings(), '3.0');
    // 2.2 is newest eligible
    expect(result).toHaveLength(2);
    expect(result[1].name).toBe('Admin');
  });
});

// ---------------------------------------------------------------------------
// parseVideoTutorials
// ---------------------------------------------------------------------------

const validRaw = {
  versions: {
    '2.2': [
      {
        name: 'Identity',
        videos: [
          {
            youtubeVideoId: 'abcDEFghiJK',
            title: 'Test video',
            description: 'A test description',
            appRoute: '/users',
            docsUrl: 'https://docs.defguard.net/users',
          },
        ],
      },
    ],
  },
};

describe('parseVideoTutorials', () => {
  it('should accept a valid contract', () => {
    const result = parseVideoTutorials(validRaw);
    expect(result['2.2']).toHaveLength(1);
    expect(result['2.2'][0].name).toBe('Identity');
    expect(result['2.2'][0].videos[0].youtubeVideoId).toBe('abcDEFghiJK');
  });

  it('should reject an invalid youtubeVideoId (not 11 chars)', () => {
    const raw = {
      versions: {
        '2.2': [
          {
            name: 'Test',
            videos: [
              {
                youtubeVideoId: 'tooshort',
                title: 'Test',
                description: 'Desc',
                appRoute: '/users',
                docsUrl: 'https://docs.defguard.net',
              },
            ],
          },
        ],
      },
    };
    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should reject an empty title', () => {
    const raw = {
      versions: {
        '2.2': [
          {
            name: 'Test',
            videos: [
              {
                youtubeVideoId: 'abcDEFghiJK',
                title: '',
                description: 'Desc',
                appRoute: '/users',
                docsUrl: 'https://docs.defguard.net',
              },
            ],
          },
        ],
      },
    };
    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should reject an empty description', () => {
    const raw = {
      versions: {
        '2.2': [
          {
            name: 'Test',
            videos: [
              {
                youtubeVideoId: 'abcDEFghiJK',
                title: 'Title',
                description: '',
                appRoute: '/users',
                docsUrl: 'https://docs.defguard.net',
              },
            ],
          },
        ],
      },
    };
    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should reject an appRoute missing a leading slash', () => {
    const raw = {
      versions: {
        '2.2': [
          {
            name: 'Test',
            videos: [
              {
                youtubeVideoId: 'abcDEFghiJK',
                title: 'Title',
                description: 'Desc',
                appRoute: 'users',
                docsUrl: 'https://docs.defguard.net',
              },
            ],
          },
        ],
      },
    };
    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should reject an invalid docsUrl', () => {
    const raw = {
      versions: {
        '2.2': [
          {
            name: 'Test',
            videos: [
              {
                youtubeVideoId: 'abcDEFghiJK',
                title: 'Title',
                description: 'Desc',
                appRoute: '/users',
                docsUrl: 'not-a-url',
              },
            ],
          },
        ],
      },
    };
    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should reject a section with an empty name', () => {
    const raw = {
      versions: {
        '2.2': [
          {
            name: '',
            videos: [],
          },
        ],
      },
    };
    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should reject an invalid version key format', () => {
    const raw = {
      versions: {
        'v2.2': [
          {
            name: 'Test',
            videos: [],
          },
        ],
      },
    };
    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should strip unknown fields from videos', () => {
    const raw = {
      versions: {
        '2.2': [
          {
            name: 'Test',
            videos: [
              {
                youtubeVideoId: 'abcDEFghiJK',
                title: 'Test',
                description: 'Desc',
                appRoute: '/users',
                docsUrl: 'https://docs.defguard.net',
                unknownField: 'ignored',
              },
            ],
          },
        ],
      },
    };
    const result = parseVideoTutorials(raw);
    expect(
      (result['2.2'][0].videos[0] as Record<string, unknown>)['unknownField'],
    ).toBeUndefined();
  });

  it('should strip unknown fields from sections', () => {
    const raw = {
      versions: {
        '2.2': [
          {
            name: 'Test',
            videos: [],
            extraSectionField: 'ignored',
          },
        ],
      },
    };
    const result = parseVideoTutorials(raw);
    expect((result['2.2'][0] as Record<string, unknown>)['extraSectionField']).toBeUndefined();
  });

  it('should reject null input', () => {
    expect(() => parseVideoTutorials(null)).toThrow();
  });

  it('should reject missing versions key', () => {
    expect(() => parseVideoTutorials({})).toThrow();
  });

  it('should accept versions with an empty sections array', () => {
    const raw = { versions: { '2.2': [] } };
    const result = parseVideoTutorials(raw);
    expect(result['2.2']).toHaveLength(0);
  });

  it('should accept a section with an empty videos array', () => {
    const raw = {
      versions: {
        '2.2': [{ name: 'Empty Section', videos: [] }],
      },
    };
    const result = parseVideoTutorials(raw);
    expect(result['2.2'][0].videos).toHaveLength(0);
  });
});
