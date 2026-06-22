import { m } from '../../../../paraglide/messages';
import {
  SMTP_OAUTH_CALLBACK_TYPE,
  SMTP_OAUTH_RESULT_KEY,
} from '../../../../routes/smtp-oauth-callback';

export const GOOGLE_ISSUER_URL = 'https://accounts.google.com';
export const MICROSOFT_ISSUER_URL = 'https://login.microsoftonline.com/common';
export const GOOGLE_AUTH_URL = `${GOOGLE_ISSUER_URL}/o/oauth2/v2/auth`;
export const GOOGLE_TOKEN_URL = 'https://oauth2.googleapis.com/token';
export const MICROSOFT_AUTH_URL = `${MICROSOFT_ISSUER_URL}/oauth2/v2.0/authorize`;
export const MICROSOFT_TOKEN_URL = `${MICROSOFT_ISSUER_URL}/oauth2/v2.0/token`;
export const CUSTOM_SCOPE_DEFAULT = 'openid offline_access';
export const GOOGLE_SMTP_SERVER = 'smtp.gmail.com';
export const GOOGLE_OAUTH_SCOPE = 'https://mail.google.com/ email';
export const MICROSOFT_SMTP_SERVER = 'smtp.office365.com';
export const PROVIDER_SMTP_PORT = 587;

export const discoverOidcEndpoints = async (
  issuerUrl: string,
): Promise<{ authorizationEndpoint: string; tokenEndpoint: string }> => {
  const discoveryUrl = `${issuerUrl.replace(/\/$/, '')}/.well-known/openid-configuration`;
  const response = await fetch(discoveryUrl);
  if (!response.ok) {
    throw new Error(m.settings_smtp_auth_oauth_error());
  }
  const data = (await response.json()) as {
    authorization_endpoint?: string;
    token_endpoint?: string;
  };
  if (!data.authorization_endpoint || !data.token_endpoint) {
    throw new Error(m.settings_smtp_auth_oauth_error());
  }
  return {
    authorizationEndpoint: data.authorization_endpoint,
    tokenEndpoint: data.token_endpoint,
  };
};

export const buildAuthUrl = (
  authorizationEndpoint: string,
  clientId: string,
  redirectUri: string,
  scope: string,
  extraParams?: Record<string, string>,
): string => {
  const params = new URLSearchParams({
    client_id: clientId,
    redirect_uri: redirectUri,
    response_type: 'code',
    scope,
    prompt: 'consent',
    ...extraParams,
  });
  return `${authorizationEndpoint}?${params.toString()}`;
};

type OAuthResult = { type?: string; code?: string; error?: string };

export const waitForOAuthCode = (popup: Window): Promise<string> =>
  new Promise((resolve, reject) => {
    const cleanup = () => {
      window.removeEventListener('message', messageHandler);
      window.removeEventListener('storage', storageHandler);
      clearTimeout(timeoutId);
      localStorage.removeItem(SMTP_OAUTH_RESULT_KEY);
    };

    const handleResult = (data: OAuthResult) => {
      if (data?.type !== SMTP_OAUTH_CALLBACK_TYPE) return;
      cleanup();
      if (data.code) {
        resolve(data.code);
      } else {
        reject(new Error(data.error ?? m.settings_smtp_auth_oauth_error()));
      }
    };

    const messageHandler = (event: MessageEvent) => {
      if (event.origin !== window.location.origin) return;
      handleResult(event.data as OAuthResult);
    };

    // COOP fallback: storage event crosses browsing-context-group boundaries.
    const storageHandler = (event: StorageEvent) => {
      if (event.key !== SMTP_OAUTH_RESULT_KEY || !event.newValue) return;
      try {
        handleResult(JSON.parse(event.newValue) as OAuthResult);
      } catch {
        // ignore malformed values
      }
    };

    localStorage.removeItem(SMTP_OAUTH_RESULT_KEY);
    window.addEventListener('message', messageHandler);
    window.addEventListener('storage', storageHandler);

    // Timeout is the only "user abandoned" signal — popup.closed is unreliable after COOP.
    const timeoutId = setTimeout(
      () => {
        try {
          popup.close();
        } catch {
          // popup may be detached after COOP
        }
        cleanup();
        reject(new Error(m.settings_smtp_auth_oauth_popup_closed()));
      },
      5 * 60 * 1000,
    );
  });

export const exchangeCodeForToken = async (
  tokenUrl: string,
  code: string,
  clientId: string,
  clientSecret: string,
  redirectUri: string,
): Promise<string> => {
  const body = new URLSearchParams({
    code,
    client_id: clientId,
    client_secret: clientSecret,
    redirect_uri: redirectUri,
    grant_type: 'authorization_code',
  });
  const response = await fetch(tokenUrl, {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: body.toString(),
  });
  if (!response.ok) {
    const error = (await response.json().catch(() => ({}))) as {
      error_description?: string;
    };
    throw new Error(error.error_description ?? m.settings_smtp_auth_oauth_error());
  }
  const data = (await response.json()) as { refresh_token?: string };
  if (!data.refresh_token) {
    throw new Error(m.settings_smtp_auth_oauth_error());
  }
  return data.refresh_token;
};
