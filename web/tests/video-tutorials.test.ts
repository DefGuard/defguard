import { describe, expect, it } from 'vitest';
import { parseVideoTutorials } from '../src/shared/video-tutorials/data';
import { matchesVideoRouteContext } from '../src/shared/video-tutorials/resolved';
import {
  resolveSections,
  resolveVideoGuidePlacement,
} from '../src/shared/video-tutorials/resolver';
import { canonicalizeRouteKey } from '../src/shared/video-tutorials/route-key';
import { parseVersion, resolveVersion } from '../src/shared/utils/resolveVersion';
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

const makeVideo = (
  id: string,
  appRoute: string,
  contextAppRoutes?: string[],
  docsUrl = 'https://docs.defguard.net/test',
) => ({
  youtubeVideoId: id,
  title: `Video ${id}`,
  description: `Description for ${id}`,
  appRoute,
  contextAppRoutes,
  docsUrl,
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
          video: {
            youtubeVideoId: 'abcDEFghiJK',
            title: 'Migration wizard guide',
          },
          docs: [
            {
              docsTitle: 'Migration wizard documentation',
              docsUrl: 'https://docs.defguard.net/migration',
            },
          ],
        },
        steps: {
          ca: {
            video: {
              youtubeVideoId: 'caGuide220a',
              title: 'Certificate authority guide',
            },
            docs: [
              {
                docsTitle: 'Certificate authority documentation',
                docsUrl: 'https://docs.defguard.net/migration/ca',
              },
            ],
          },
        },
      },
      initialSetupWizard: {
        default: {
          video: {
            youtubeVideoId: 'setGuide220',
            title: 'Initial setup guide',
          },
          docs: [
            {
              docsTitle: 'Initial setup documentation',
              docsUrl: 'https://docs.defguard.net/setup',
            },
          ],
        },
        steps: {
          adminUser: {
            video: {
              youtubeVideoId: 'admGuide220',
              title: 'Admin user guide',
            },
            docs: [
              {
                docsTitle: 'Admin user documentation',
                docsUrl: 'https://docs.defguard.net/setup/admin-user',
              },
            ],
          },
        },
      },
      autoAdoptionWizard: {
        default: {
          video: {
            youtubeVideoId: 'autoGuid220',
            title: 'Auto adoption guide',
          },
          docs: [
            {
              docsTitle: 'Auto adoption documentation',
              docsUrl: 'https://docs.defguard.net/auto-adoption',
            },
          ],
        },
        steps: {
          vpnSettings: {
            video: {
              youtubeVideoId: 'vpnGuide220',
              title: 'VPN settings guide',
            },
            docs: [
              {
                docsTitle: 'VPN settings documentation',
                docsUrl: 'https://docs.defguard.net/auto-adoption/vpn-settings',
              },
            ],
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
    expect(result?.placements?.migrationWizard?.default?.video?.youtubeVideoId).toBe(
      'abcDEFghiJK',
    );
    expect(result?.placements?.initialSetupWizard?.default?.video?.youtubeVideoId).toBe(
      'setGuide220',
    );
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
// matchesVideoRouteContext
// ---------------------------------------------------------------------------

describe('matchesVideoRouteContext', () => {
  it('should match the primary appRoute', () => {
    const video = makeVideo('abcDEFghiJK', '/users');

    expect(matchesVideoRouteContext(video, '/users')).toBe(true);
  });

  it('should match a contextAppRoutes entry', () => {
    const video = makeVideo('abcDEFghiJK', '/users', ['/groups', '/settings/']);

    expect(matchesVideoRouteContext(video, '/settings')).toBe(true);
  });

  it('should return false when neither appRoute nor contextAppRoutes match', () => {
    const video = makeVideo('abcDEFghiJK', '/users', ['/groups']);

    expect(matchesVideoRouteContext(video, '/settings')).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// resolveVideoGuidePlacement
// ---------------------------------------------------------------------------

describe('resolveVideoGuidePlacement', () => {
  it('should return the step-specific placement from the newest eligible version', () => {
    const result = resolveVideoGuidePlacement(makeMappings(), '2.3.0', 'migrationWizard', 'ca');

    expect(result?.video?.title).toBe('Certificate authority guide');
  });

  it('should fall back to the default placement when step-specific entry is missing', () => {
    const result = resolveVideoGuidePlacement(
      makeMappings(),
      '2.3.0',
      'migrationWizard',
      'general',
    );

    expect(result?.video?.title).toBe('Migration wizard guide');
  });

  it('should not fall back to an older placement once a newer eligible version is selected', () => {
    const mappings: VideoTutorialsMappings = {
      '2.1': {
        sections: [],
        placements: {
          migrationWizard: {
            default: {
              video: {
                youtubeVideoId: 'abcDEFghiJK',
                title: 'Migration wizard guide',
              },
              docs: [
                {
                  docsTitle: 'Migration wizard documentation',
                  docsUrl: 'https://docs.defguard.net/migration',
                },
              ],
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

  it('should resolve a step-specific placement for initial setup wizard', () => {
    const result = resolveVideoGuidePlacement(
      makeMappings(),
      '2.3.0',
      'initialSetupWizard',
      'adminUser',
    );

    expect(result?.video?.title).toBe('Admin user guide');
  });

  it('should resolve a default placement for auto adoption wizard when step is missing', () => {
    const result = resolveVideoGuidePlacement(
      makeMappings(),
      '2.3.0',
      'autoAdoptionWizard',
      'summary',
    );

    expect(result?.video?.title).toBe('Auto adoption guide');
  });

  it('should resolve a step-specific placement for auto adoption wizard', () => {
    const result = resolveVideoGuidePlacement(
      makeMappings(),
      '2.3.0',
      'autoAdoptionWizard',
      'vpnSettings',
    );

    expect(result?.video?.title).toBe('VPN settings guide');
  });

  it('should not merge a step placement with the default placement', () => {
    const mappings: VideoTutorialsMappings = {
      '2.2': {
        sections: [],
        placements: {
          migrationWizard: {
            default: {
              video: {
                youtubeVideoId: 'abcDEFghiJK',
                title: 'Migration wizard guide',
              },
              docs: [
                {
                  docsTitle: 'Migration wizard documentation',
                  docsUrl: 'https://docs.defguard.net/migration',
                },
              ],
            },
            steps: {
              ca: {
                docs: [
                  {
                    docsTitle: 'Certificate authority documentation',
                    docsUrl: 'https://docs.defguard.net/migration/ca',
                  },
                ],
              },
            },
          },
        },
      },
    };

    const result = resolveVideoGuidePlacement(mappings, '2.2', 'migrationWizard', 'ca');

    expect(result?.video).toBeUndefined();
    expect(result?.docs).toEqual([
      {
        docsTitle: 'Certificate authority documentation',
        docsUrl: 'https://docs.defguard.net/migration/ca',
      },
    ]);
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
            video: {
              youtubeVideoId: 'xyz987GHI12',
              title: 'Migration guide',
            },
            docs: [
              {
                docsTitle: 'Defguard Configuration Guide',
                docsUrl: 'https://docs.defguard.net/migration',
              },
            ],
          },
          steps: {
            general: {
              video: {
                youtubeVideoId: 'genGuide220',
                title: 'General configuration guide',
              },
              docs: [
                {
                  docsTitle: 'General configuration documentation',
                  docsUrl: 'https://docs.defguard.net/migration/general',
                },
              ],
            },
          },
        },
        initialSetupWizard: {
          default: {
            video: {
              youtubeVideoId: 'setGuide220',
              title: 'Setup guide',
            },
            docs: [
              {
                docsTitle: 'Setup docs',
                docsUrl: 'https://docs.defguard.net/setup',
              },
            ],
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
    expect(result['2.2'].placements?.migrationWizard?.default?.video?.youtubeVideoId).toBe(
      'xyz987GHI12',
    );
    expect(result['2.2'].placements?.migrationWizard?.default?.docs?.[0]?.docsTitle).toBe(
      'Defguard Configuration Guide',
    );
    expect(result['2.2'].placements?.migrationWizard?.steps?.general?.video?.youtubeVideoId).toBe(
      'genGuide220',
    );
    expect(result['2.2'].placements?.initialSetupWizard?.default?.video?.youtubeVideoId).toBe(
      'setGuide220',
    );
  });

  it('should accept missing docsUrl', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [
            {
              name: 'Test',
              videos: [
                {
                  youtubeVideoId: 'abcDEFghiJK',
                  title: 'Test video',
                  description: 'A test description',
                  appRoute: '/users',
                },
              ],
            },
          ],
        },
      },
    };

    const result = parseVideoTutorials(raw);

    expect(result['2.2'].sections[0].videos[0].docsUrl).toBeUndefined();
  });

  it('should accept contextAppRoutes when present', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [
            {
              name: 'Test',
              videos: [
                {
                  youtubeVideoId: 'abcDEFghiJK',
                  title: 'Test video',
                  description: 'A test description',
                  appRoute: '/users',
                  contextAppRoutes: ['/groups', '/settings/'],
                  docsUrl: 'https://docs.defguard.net/users',
                },
              ],
            },
          ],
        },
      },
    };

    const result = parseVideoTutorials(raw);

    expect(result['2.2'].sections[0].videos[0].contextAppRoutes).toEqual([
      '/groups',
      '/settings/',
    ]);
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

  it('should reject contextAppRoutes entries missing a leading slash', () => {
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
                  contextAppRoutes: ['groups'],
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

  it('should reject an empty contextAppRoutes array', () => {
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
                  contextAppRoutes: [],
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

  it('should preserve contextAppRoutes while stripping unknown video fields', () => {
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
                  contextAppRoutes: ['/groups'],
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

    expect(result['2.2'].sections[0].videos[0].contextAppRoutes).toEqual(['/groups']);
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

  it('should reject invalid placement docsUrl', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [],
          placements: {
            initialSetupWizard: {
              default: {
                docs: [
                  {
                    docsTitle: 'Setup Guide',
                    docsUrl: 'not-a-url',
                  },
                ],
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
                video: {
                  youtubeVideoId: 'xyz987GHI12',
                  title: 'Migration guide',
                  ignoredVideoField: 'ignored',
                },
                docs: [
                  {
                    docsTitle: 'Defguard Configuration Guide',
                    docsUrl: 'https://docs.defguard.net/migration',
                    ignoredDocsField: 'ignored',
                  },
                ],
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
    expect(
      (result['2.2'].placements?.migrationWizard?.default?.video as Record<string, unknown>)[
        'ignoredVideoField'
      ],
    ).toBeUndefined();
    expect(
      (result['2.2'].placements?.migrationWizard?.default?.docs?.[0] as Record<string, unknown>)[
        'ignoredDocsField'
      ],
    ).toBeUndefined();
  });

  it('should reject an empty placement docsTitle', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [],
          placements: {
            autoAdoptionWizard: {
              default: {
                docs: [
                  {
                    docsTitle: '',
                    docsUrl: 'https://docs.defguard.net/auto-adoption',
                  },
                ],
              },
            },
          },
        },
      },
    };

    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should reject invalid generic placement step docsUrl', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [],
          placements: {
            autoAdoptionWizard: {
              steps: {
                vpnSettings: {
                  docs: [
                    {
                      docsTitle: 'VPN settings guide',
                      docsUrl: 'not-a-url',
                    },
                  ],
                },
              },
            },
          },
        },
      },
    };

    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should accept generic placement keys', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [],
          placements: {
            anyWizardKey: {
              default: {
                video: {
                  youtubeVideoId: 'xyz987GHI12',
                  title: 'Generic guide',
                },
                docs: [
                  {
                    docsTitle: 'Generic docs',
                    docsUrl: 'https://docs.defguard.net/generic',
                  },
                ],
              },
            },
          },
        },
      },
    };

    const result = parseVideoTutorials(raw);

    expect(result['2.2'].placements?.anyWizardKey?.default?.video?.title).toBe(
      'Generic guide',
    );
  });

  it('should accept a placement with only video', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [],
          placements: {
            anyWizardKey: {
              default: {
                video: {
                  youtubeVideoId: 'xyz987GHI12',
                  title: 'Generic guide',
                },
              },
            },
          },
        },
      },
    };

    const result = parseVideoTutorials(raw);

    expect(result['2.2'].placements?.anyWizardKey?.default).toEqual({
      video: {
        youtubeVideoId: 'xyz987GHI12',
        title: 'Generic guide',
      },
    });
  });

  it('should accept a placement with only docs', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [],
          placements: {
            anyWizardKey: {
              default: {
                docs: [
                  {
                    docsTitle: 'Generic docs',
                    docsUrl: 'https://docs.defguard.net/generic',
                  },
                ],
              },
            },
          },
        },
      },
    };

    const result = parseVideoTutorials(raw);

    expect(result['2.2'].placements?.anyWizardKey?.default).toEqual({
      docs: [
        {
          docsTitle: 'Generic docs',
          docsUrl: 'https://docs.defguard.net/generic',
        },
      ],
    });
  });

  it('should accept multiple docs links in a placement', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [],
          placements: {
            anyWizardKey: {
              default: {
                docs: [
                  {
                    docsTitle: 'Generic docs',
                    docsUrl: 'https://docs.defguard.net/generic',
                  },
                  {
                    docsTitle: 'More docs',
                    docsUrl: 'https://docs.defguard.net/generic/more',
                  },
                ],
              },
            },
          },
        },
      },
    };

    const result = parseVideoTutorials(raw);

    expect(result['2.2'].placements?.anyWizardKey?.default?.docs).toHaveLength(2);
  });

  it('should reject an empty docs array when docs is present', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [],
          placements: {
            anyWizardKey: {
              default: {
                docs: [],
              },
            },
          },
        },
      },
    };

    expect(() => parseVideoTutorials(raw)).toThrow();
  });

  it('should reject a placement with neither video nor docs', () => {
    const raw = {
      versions: {
        '2.2': {
          sections: [],
          placements: {
            anyWizardKey: {
              default: {},
            },
          },
        },
      },
    };

    expect(() => parseVideoTutorials(raw)).toThrow();
  });
});
