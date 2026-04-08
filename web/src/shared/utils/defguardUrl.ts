import ipaddr from 'ipaddr.js';

import type { ExternalSslType, InternalSslType } from '../api/types';

/**
 * Ensures a URL string has a scheme (`http://` or `https://`).
 * If the value already has a valid scheme it is returned unchanged.
 * If it has no recognised scheme, `https://` is prepended.
 *
 * Intended for use as a Zod `.overwrite()` callback so that bare
 * hostnames typed by the user (e.g. `example.com`) are accepted and
 * treated as HTTPS URLs before further validation runs.
 */
export const ensureUrlScheme = (val: string): string => {
  try {
    new URL(val);
    return val; // already has a valid scheme
  } catch {
    return `https://${val}`;
  }
};

/**
 * Corrects the URL scheme to match the given SSL type.
 * - `none` -> left unchanged (user may be behind a reverse proxy handling SSL)
 * - anything else -> forces `https://`
 *
 * Falls back to prepending the scheme when parsing fails.
 */
export const correctUrlProtocol = (
  url: string,
  sslType: InternalSslType | ExternalSslType,
): string => {
  if (sslType === 'none') return url;
  try {
    const parsed = new URL(url);
    parsed.protocol = 'https:';
    return parsed.toString();
  } catch {
    return `https://${url}`;
  }
};

export const isValidDefguardUrl = (value: string): boolean => {
  try {
    const url = new URL(value);
    const hostname = url.hostname;

    if (hostname.length === 0) {
      return false;
    }

    if (ipaddr.isValid(hostname)) {
      return false;
    }

    return true;
  } catch {
    return false;
  }
};
