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
 * - `none` -> forces `http://`
 * - anything else -> forces `https://`
 *
 * Falls back to prepending the scheme when parsing fails.
 */
export const correctUrlProtocol = (
  url: string,
  sslType: InternalSslType | ExternalSslType,
): string => {
  const protocol: 'http:' | 'https:' = sslType === 'none' ? 'http:' : 'https:';
  try {
    const parsed = new URL(url);
    parsed.protocol = protocol;
    return parsed.toString();
  } catch {
    return `${protocol}//${url}`;
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
