import { describe, expect, it } from 'vitest';
import { parseVideoTutorials } from '../src/shared/video-tutorials/data';
import {
  resolveSections,
  resolveVideoGuidePlacement,
  resolveVersion,
} from '../src/shared/video-tutorials/resolver';
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
// Shared fixture helpers
// ---------------------------------------------------------------------------

const makeVideo = (id: string, appRoute: string) => ({
  youtubeVideoId: id,
  title: `Video ${id}`,
  description: `Description for ${id}`,
  appRoute,
  docsUrl: 'https://docs.defguard.net/test',
});

const makeMappings = (): VideoTutorialsMappings => ({
  '2.0': {
    sections: [
      {
        name: 'Identity',
        videos: [makeVideo('usrGuide200', '/users')],
      },
    ],
  },
  '2.2': {
    sections: [
      {
        name: 'Identity',
        videos: [makeVideo('usrGuide220', '/users')],
      },
      {
        name: 'Admin',
        videos: [makeVideo('setGuide220', '/settings')],
      },
    ],
    placements: {
      migrationWizard: {
        default: {
          youtubeVideoId: 'abcDEFghiJK',
          title: 'Migration wizard guide',
          docsTitle: 'Migration wizard documentation',
          docsUrl: 'https://docs.defguard.net/migration',
        },
        steps: {
          ca: {
            youtubeVideoId: 'caGuide220a',
            title: 'Certificate authority guide',
            docsTitle: 'Certificate authority documentation',
            docsUrl: 'https://docs.defguard.net/migration/ca',
          },
        },
      },
    },
  },
});

// ---------------------------------------------------------------------------
// resolveVersion
// ---------------------------------------------------------------------------

describe('resolveVersion', () => {
  it('should return the newest eligible version entry', () => {
    const result = resolveVersion(makeMappings(), '2.3.0');

    expect(result?.sections).toHaveLength(2);
    expect(result?.placements?.migrationWizard?.default?.youtubeVideoId).toBe('abcDEFghiJK');
  });

  it('should return null for an unparseable app version', () => {
    expect(resolveVersion(makeMappings(), '')).toBeNull();
  });

  it('should strip prerelease from runtime version before resolving', () => {
    const result = resolveVersion(makeMappings(), '2.2.0-beta');

    expect(result?.sections).toHaveLength(2);
    expect(result?.sections[0].videos[0].youtubeVideoId).toBe('usrGuide220');
  });
});

// ---------------------------------------------------------------------------
// resolveSections
// ---------------------------------------------------------------------------

describe('resolveSections', () => {
  it('should return all sections from the newest eligible version', () => {
    const result = resolveSections(makeMappings(), '2.2');

    expect(result).toHaveLength(2);
    expect(result[0].name).toBe('Identity');
    expect(result[1].name).toBe('Admin');
  });

  it('should respect the app version ceiling (not use versions newer than app)', () => {
    const result = resolveSections(makeMappings(), '2.0');

    expect(result).toHaveLength(1);
    expect(result[0].name).toBe('Identity');
  });

  it('should return empty array for an unparseable app version', () => {
    expect(resolveSections(makeMappings(), '')).toHaveLength(0);
  });

  it('should return empty array when no eligible version exists', () => {
    expect(resolveSections(makeMappings(), '1.0')).toHaveLength(0);
  });

  it('should pick the newest when multiple versions are eligible', () => {
    const result = resolveSections(makeMappings(), '3.0');

    expect(result).toHaveLength(2);
    expect(result[1].name).toBe('Admin');
  });
});

// ---------------------------------------------------------------------------
// resolveVideoGuidePlacement
// ---------------------------------------------------------------------------

describe('resolveVideoGuidePlacement', () => {
  it('should return the step-specific placement from the newest eligible version', () => {
    const result = resolveVideoGuidePlacement(makeMappings(), '2.3.0', 'migrationWizard', 'ca');

    expect(result?.title).toBe('Certificate authority guide');
  });

  it('should fall back to the default placement when step-specific entry is missing', () => {
    const result = resolveVideoGuidePlacement(
      makeMappings(),
      '2.3.0',
      'migrationWizard',
      'general',
    );

    expect(result?.title).toBe('Migration wizard guide');
  });

  it('should not fall back to an older placement once a newer eligible version is selected', () => {
    const mappings: VideoTutorialsMappings = {
      '2.1': {
        sections: [],
        placements: {
          migrationWizard: {
            default: {
              youtubeVideoId: 'abcDEFghiJK',
              title: 'Migration wizard guide',
              docsTitle: 'Migration wizard documentation',
              docsUrl: 'https://docs.defguard.net/migration',
            },
          },
        },
      },
      '2.2': {
        sections: [],
      },
    };

    const result = resolveVideoGuidePlacement(mappings, '2.2', 'migrationWizard', 'ca');

    expect(result).toBeNull();
  });

  it('should return null when the selected version has no placement', () => {
    const mappings: VideoTutorialsMappings = {
      '2.2': {
        sections: [],
      },
    };

    expect(resolveVideoGuidePlacement(mappings, '2.2', 'migrationWizard', 'ca')).toBeNull();
  });

  it('should return null when neither default nor step-specific placement exists', () => {
    const mappings: VideoTutorialsMappings = {
      '2.2': {
        sections: [],
        placements: {
          migrationWizard: {
            steps: {},
          },
        },
      },
    };

    expect(resolveVideoGuidePlacement(mappings, '2.2', 'migrationWizard', 'ca')).toBeNull();
  });

  it('should return null for an unsupported placement key', () => {
    expect(
      resolveVideoGuidePlacement(makeMappings(), '2.3.0', 'unknownPlacement', 'ca'),
    ).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// parseVideoTutorials
// ---------------------------------------------------------------------------

const validRaw = {
  versions: {
    '2.2': {
      sections: [
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
      placements: {
        migrationWizard: {
          default: {
            youtubeVideoId: 'xyz987GHI12',
            title: 'Migration guide',
            docsTitle: 'Defguard Configuration Guide',
            docsUrl: 'https://docs.defguard.net/migration',
          },
          steps: {
            general: {
              youtubeVideoId: 'genGuide220a',
              title: 'General configuration guide',
              docsTitle: 'General configuration documentation',
              docsUrl: 'https://docs.defguard.net/migration/general',
            },
          },
        },
      },
    },
  },
};

describe('parseVideoTutorials', () => {
  it('should accept a valid contract', () => {
    const result = parseVideoTutorials(validRaw);

    expect(result['2.2'].sections).toHaveLength(1);
    expect(result['2.2'].sections[0].name).toBe('Identity');
    expect(result['2.2'].sections[0].videos[0].youtubeVideoId).toBe('abcDEFghiJK');
    expect(result['2.2'].placements?.migrationWizard?.default?.youtubeVideoId).toBe(
      'xyz987GHI12',
    );
    expect(result['2.2'].placements?.migrationWizard?.default?.docsTitle).toBe(
      'Defguard Configuration Guide',
    );
    expect(result['2.2'].placements?.migrationWizard?.steps?.general?.youtubeVideoId).toBe(
      'genGuide220a',
    );
  });

  it('should reject an invalid youtubeVideoId (not 11 chars)', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [
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
      },
    };
    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should reject an empty title', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [
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
      },
    };
    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should reject an empty description', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [
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
      },
    };
    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should reject an appRoute missing a leading slash', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [
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
      },
    };
    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should reject an invalid docsUrl', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [
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
      },
    };
    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should reject a section with an empty name', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [
            {
              name: '',
              videos: [],
            },
          ],
        },
      },
    };
    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should reject an invalid version key format', () => {
    const raw = {
      versions: {
        'v2.2': {
          sections: [],
        },
      },
    };
    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should strip unknown fields from videos', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [
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
      },
    };
    const result = parseVideoTutorials(raw);
    expect(
      (result['2.2'].sections[0].videos[0] as Record<string, unknown>)['unknownField'],
    ).toBeUndefined();
  });

  it('should strip unknown fields from sections', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [
            {
              name: 'Test',
              videos: [],
              extraSectionField: 'ignored',
            },
          ],
        },
      },
    };
    const result = parseVideoTutorials(raw);
    expect(
      (result['2.2'].sections[0] as Record<string, unknown>)['extraSectionField'],
    ).toBeUndefined();
  });

  it('should reject null input', () => {
    expect(() => parseVideoTutorials(null)).toThrow();
  });

  it('should reject missing versions key', () => {
    expect(() => parseVideoTutorials({})).toThrow();
  });

  it('should require sections to be present', () => {
    const raw = {
      versions: {
        '2.2': {
          placements: {},
        },
      },
    };

    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should accept versions with an empty sections array', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [],
        },
      },
    };
    const result = parseVideoTutorials(raw);
    expect(result['2.2'].sections).toHaveLength(0);
  });

  it('should accept a section with an empty videos array', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [{ name: 'Empty Section', videos: [] }],
        },
      },
    };
    const result = parseVideoTutorials(raw);
    expect(result['2.2'].sections[0].videos).toHaveLength(0);
  });

  it('should reject invalid migrationWizard docsUrl', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [],
          placements: {
            migrationWizard: {
              default: {
                youtubeVideoId: 'xyz987GHI12',
                title: 'Migration guide',
                docsTitle: 'Defguard Configuration Guide',
                docsUrl: 'not-a-url',
              },
            },
          },
        },
      },
    };

    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should strip unknown fields from version entries and placements', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [],
          extraVersionField: 'ignored',
          placements: {
            migrationWizard: {
              default: {
                youtubeVideoId: 'xyz987GHI12',
                title: 'Migration guide',
                docsTitle: 'Defguard Configuration Guide',
                docsUrl: 'https://docs.defguard.net/migration',
              },
              extraPlacementField: 'ignored',
            },
          },
        },
      },
    };

    const result = parseVideoTutorials(raw);

    expect((result['2.2'] as Record<string, unknown>)['extraVersionField']).toBeUndefined();
    expect(
      (result['2.2'].placements?.migrationWizard as Record<string, unknown>)['extraPlacementField'],
    ).toBeUndefined();
  });

  it('should reject an empty migrationWizard docsTitle', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [],
          placements: {
            migrationWizard: {
              default: {
                youtubeVideoId: 'xyz987GHI12',
                title: 'Migration guide',
                docsTitle: '',
                docsUrl: 'https://docs.defguard.net/migration',
              },
            },
          },
        },
      },
    };

    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should reject invalid migrationWizard step docsUrl', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [],
          placements: {
            migrationWizard: {
              steps: {
                ca: {
                  youtubeVideoId: 'xyz987GHI12',
                  title: 'Migration guide',
                  docsTitle: 'Certificate authority guide',
                  docsUrl: 'not-a-url',
                },
              },
            },
          },
        },
      },
    };

    expect(() => parseVideoTutorials(raw)).toThrow();
  });
});
