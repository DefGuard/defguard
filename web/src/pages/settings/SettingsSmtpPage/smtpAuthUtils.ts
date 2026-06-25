const safeHostname = (url: string | null | undefined): string | null => {
  if (!url) return null;
  try {
    return new URL(url).hostname;
  } catch {
    return null;
  }
};

export const isGoogleIssuerUrl = (url: string | null | undefined): boolean =>
  safeHostname(url) === 'accounts.google.com';

export const isMicrosoftIssuerUrl = (url: string | null | undefined): boolean => {
  const hostname = safeHostname(url);
  return (
    hostname === 'microsoftonline.com' ||
    Boolean(hostname?.endsWith('.microsoftonline.com'))
  );
};
