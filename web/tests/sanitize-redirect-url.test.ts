import { describe, expect, it } from 'vitest';
import { sanitizeRedirectUrl } from '../src/shared/utils/sanitizeRedirectUrl';

describe('sanitizeRedirectUrl', () => {
  it('accepts absolute http(s) urls', () => {
    expect(sanitizeRedirectUrl('https://whoami.example.com/path?a=1')).toBe(
      'https://whoami.example.com/path?a=1',
    );
    expect(sanitizeRedirectUrl('http://app.example.com/')).toBe(
      'http://app.example.com/',
    );
  });

  it.each([
    undefined,
    '',
    'javascript:alert(1)',
    'data:text/html,hi',
    '/relative/path',
    '//evil.example.com',
    'https://user:pass@example.com/',
    'https:\\\\example.com',
    'https://example.com/a b',
  ])('rejects %s', (value) => {
    expect(sanitizeRedirectUrl(value)).toBeUndefined();
  });
});
