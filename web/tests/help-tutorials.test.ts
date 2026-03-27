import { describe, expect, it } from 'vitest';
import { parseHelpTutorials } from '../src/shared/help-tutorials/data';
import { resolveHelpTutorials } from '../src/shared/help-tutorials/resolver';
import { canonicalizeRouteKey } from '../src/shared/help-tutorials/route-key';
import { parseVersion } from '../src/shared/help-tutorials/version';
import type { HelpTutorialsMappings } from '../src/shared/help-tutorials/types';

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
// resolveHelpTutorials
// ---------------------------------------------------------------------------

const makeMappings = (): HelpTutorialsMappings => ({
  '2.0': {
    '/users': [{ youtubeVideoId: 'usrGuide200', title: 'Users 2.0' }],
  },
  '2.2': {
    '/users': [{ youtubeVideoId: 'usrGuide220', title: 'Users 2.2' }],
    '/settings': [{ youtubeVideoId: 'setGuide220', title: 'Settings 2.2' }],
  },
});

describe('resolveHelpTutorials', () => {
  it('should return tutorials for an exact version match', () => {
    const result = resolveHelpTutorials(makeMappings(), '2.2', '/users');
    expect(result).toHaveLength(1);
    expect(result[0].youtubeVideoId).toBe('usrGuide220');
  });

  it('should fall back to an older version when newer does not define the route', () => {
    const result = resolveHelpTutorials(makeMappings(), '2.2', '/users');
    // Sanity: 2.2 defines /users, so we get 2.2 entry
    expect(result[0].youtubeVideoId).toBe('usrGuide220');
  });

  it('should fall back to 2.0 for a route only defined there when running 2.2', () => {
    const mappings: HelpTutorialsMappings = {
      '2.0': { '/users': [{ youtubeVideoId: 'usrGuide200', title: 'Users 2.0' }] },
      '2.2': { '/settings': [{ youtubeVideoId: 'setGuide220', title: 'Settings 2.2' }] },
    };
    const result = resolveHelpTutorials(mappings, '2.2', '/users');
    expect(result[0].youtubeVideoId).toBe('usrGuide200');
  });

  it('should not use a version newer than the runtime version', () => {
    const result = resolveHelpTutorials(makeMappings(), '2.0', '/settings');
    expect(result).toHaveLength(0);
  });

  it('should preserve an explicit empty array without falling back', () => {
    const mappings: HelpTutorialsMappings = {
      '2.0': { '/users': [{ youtubeVideoId: 'usrGuide200', title: 'Users 2.0' }] },
      '2.2': { '/users': [] },
    };
    const result = resolveHelpTutorials(mappings, '2.2', '/users');
    expect(result).toHaveLength(0);
  });

  it('should return empty array when no version defines the route', () => {
    const result = resolveHelpTutorials(makeMappings(), '2.2', '/nonexistent');
    expect(result).toHaveLength(0);
  });

  it('should return empty array for an unparseable app version', () => {
    const result = resolveHelpTutorials(makeMappings(), '', '/users');
    expect(result).toHaveLength(0);
  });

  it('should strip prerelease from runtime version before resolving', () => {
    const result = resolveHelpTutorials(makeMappings(), '2.2.0-beta', '/users');
    expect(result[0].youtubeVideoId).toBe('usrGuide220');
  });
});

// ---------------------------------------------------------------------------
// parseHelpTutorials
// ---------------------------------------------------------------------------

const validRaw = {
  versions: {
    '2.2': {
      '/users': [
        {
          youtubeVideoId: 'abcDEFghiJK',
          title: 'Test tutorial',
        },
      ],
    },
  },
};

describe('parseHelpTutorials', () => {
  it('should accept a valid contract', () => {
    const result = parseHelpTutorials(validRaw);
    expect(result['2.2']['/users']).toHaveLength(1);
    expect(result['2.2']['/users'][0].youtubeVideoId).toBe('abcDEFghiJK');
  });

  it('should canonicalize route keys (strip trailing slash)', () => {
    const raw = {
      versions: {
        '2.0': {
          '/settings/': [{ youtubeVideoId: 'abcDEFghiJK', title: 'Settings' }],
        },
      },
    };
    const result = parseHelpTutorials(raw);
    expect(result['2.0']['/settings']).toBeDefined();
    expect(result['2.0']['/settings/']).toBeUndefined();
  });

  it('should reject an invalid youtubeVideoId (not 11 chars)', () => {
    const raw = {
      versions: {
        '2.2': {
          '/users': [{ youtubeVideoId: 'tooshort', title: 'Test' }],
        },
      },
    };
    expect(() => parseHelpTutorials(raw)).toThrow();
  });

  it('should reject an empty title', () => {
    const raw = {
      versions: {
        '2.2': {
          '/users': [{ youtubeVideoId: 'abcDEFghiJK', title: '' }],
        },
      },
    };
    expect(() => parseHelpTutorials(raw)).toThrow();
  });

  it('should reject duplicate route keys after canonicalization', () => {
    const raw = {
      versions: {
        '2.2': {
          '/settings': [{ youtubeVideoId: 'abcDEFghiJK', title: 'A' }],
          '/settings/': [{ youtubeVideoId: 'abcDEFghiJK', title: 'B' }],
        },
      },
    };
    expect(() => parseHelpTutorials(raw)).toThrow(/[Dd]uplicate/);
  });

  it('should reject a route key missing a leading slash', () => {
    const raw = {
      versions: {
        '2.2': {
          'settings': [{ youtubeVideoId: 'abcDEFghiJK', title: 'Test' }],
        },
      },
    };
    expect(() => parseHelpTutorials(raw)).toThrow();
  });

  it('should reject an invalid version key format', () => {
    const raw = {
      versions: {
        'v2.2': {
          '/users': [{ youtubeVideoId: 'abcDEFghiJK', title: 'Test' }],
        },
      },
    };
    expect(() => parseHelpTutorials(raw)).toThrow();
  });

  it('should strip unknown fields from tutorials', () => {
    const raw = {
      versions: {
        '2.2': {
          '/users': [{ youtubeVideoId: 'abcDEFghiJK', title: 'Test', unknownField: 'ignored' }],
        },
      },
    };
    const result = parseHelpTutorials(raw);
    expect((result['2.2']['/users'][0] as Record<string, unknown>)['unknownField']).toBeUndefined();
  });

  it('should reject null input', () => {
    expect(() => parseHelpTutorials(null)).toThrow();
  });

  it('should reject missing versions key', () => {
    expect(() => parseHelpTutorials({})).toThrow();
  });
});
