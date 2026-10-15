import { describe, expect, it } from 'vitest';
import z from 'zod';
import { ensureUrlScheme } from '../src/shared/utils/defguardUrl';
import { createDistinctUrlRefinement } from '../src/shared/utils/defguardUrlValidation';

const createUrlSettingsSchema = (getOtherUrl: () => string | null) => {
  return z
    .object({
      defguard_url: z.string().overwrite(ensureUrlScheme).url(),
      ssl_type: z.string(),
    })
    .superRefine(
      createDistinctUrlRefinement(getOtherUrl, 'defguard_url', 'URLs must differ'),
    );
};

describe('URL settings validation', () => {
  it('accepts different URLs and rejects equal addresses on the edited field', () => {
    const schema = createUrlSettingsSchema(() => 'https://core.example');
    const equal = schema.safeParse({ defguard_url: 'core.example', ssl_type: 'none' });
    const different = schema.safeParse({
      defguard_url: 'other.example',
      ssl_type: 'none',
    });

    expect(equal.success).toBe(false);
    if (!equal.success) expect(equal.error.issues[0].path).toEqual(['defguard_url']);
    expect(different.success).toBe(true);
  });

  it('normalizes bare hostnames and applies the selected SSL scheme', () => {
    const schema = createUrlSettingsSchema(() => 'https://core.example/');
    expect(ensureUrlScheme('core.example')).toBe('https://core.example');
    expect(
      schema.safeParse({ defguard_url: 'core.example', ssl_type: 'none' }).success,
    ).toBe(false);

    const sslSchema = createUrlSettingsSchema(() => 'https://core.example/');
    expect(
      sslSchema.safeParse({
        defguard_url: 'http://core.example',
        ssl_type: 'defguard_ca',
      }).success,
    ).toBe(false);
  });

  it('validates against the other URL currently stored after returning to edit', () => {
    let storedOtherUrl: string | null = 'https://old-edge.example/';
    const schema = createUrlSettingsSchema(() => storedOtherUrl);

    expect(
      schema.safeParse({ defguard_url: 'core.example', ssl_type: 'none' }).success,
    ).toBe(true);

    storedOtherUrl = 'https://core.example';
    expect(
      schema.safeParse({ defguard_url: 'core.example', ssl_type: 'none' }).success,
    ).toBe(false);
  });
});
