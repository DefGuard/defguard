// Only absolute http(s) URLs without credentials are accepted, so the value
// from the `r` query parameter (forward auth) cannot be used for `javascript:`
// or similar schemes.
export const sanitizeRedirectUrl = (value?: string | null): string | undefined => {
  if (!value || /[\\\s]/.test(value)) return undefined;
  try {
    const url = new URL(value);
    if (url.protocol !== 'http:' && url.protocol !== 'https:') return undefined;
    if (url.username || url.password) return undefined;
    return url.href;
  } catch {
    return undefined;
  }
};
