import { describe, expect, it } from 'vitest';
import z from 'zod';
import { formSchema } from '../src/pages/settings/SettingsInstancePage/SettingsInstancePage';
import { ensureUrlScheme } from '../src/shared/utils/defguardUrl';
import {
  canonicalizeUrl,
  createDistinctUrlRefinement,
} from '../src/shared/utils/defguardUrlValidation';

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

describe('Settings instance URL validation', () => {
  const values = {
    defguard_url: 'https://core.example/',
    instance_name: 'Example instance',
    public_proxy_url: 'https://core.example',
    authentication_period_days: 7,
    enable_stats_purge: true,
    stats_purge_frequency_hours: 24,
    stats_purge_threshold_days: 30,
    password_reset_token_timeout_hours: 24,
    password_reset_session_timeout_minutes: 10,
  };

  it.each([
    ['root trailing slash', 'https://core.example/', 'https://core.example'],
    ['explicit default port', 'https://core.example:443', 'https://core.example'],
  ])('canonicalizes %s', (_description, first, second) => {
    expect(canonicalizeUrl(first)).toBe(canonicalizeUrl(second));
  });

  it('reports equivalent defguard and proxy URLs on both fields', () => {
    const result = formSchema.safeParse(values);
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(
        result.error.issues
          .filter(
            (issue) =>
              issue.code === 'custom' &&
              ['defguard_url', 'public_proxy_url'].includes(String(issue.path[0])),
          )
          .map((issue) => issue.path[0]),
      ).toEqual(['defguard_url', 'public_proxy_url']);
    }
  });

  it('accepts distinct URLs in the settings schema', () => {
    expect(
      formSchema.safeParse({ ...values, public_proxy_url: 'https://proxy.example' })
        .success,
    ).toBe(true);
  });
});

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
