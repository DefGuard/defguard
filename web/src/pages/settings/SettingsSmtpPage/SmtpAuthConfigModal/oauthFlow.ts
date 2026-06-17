import { m } from '../../../../paraglide/messages';
import { SMTP_OAUTH_CALLBACK_TYPE } from '../../../../routes/smtp-oauth-callback';

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

export const waitForOAuthCode = (popup: Window): Promise<string> =>
  new Promise((resolve, reject) => {
    const messageHandler = (event: MessageEvent) => {
      if (event.origin !== window.location.origin) return;
      const data = event.data as {
        type?: string;
        code?: string;
        error?: string;
      };
      if (data?.type !== SMTP_OAUTH_CALLBACK_TYPE) return;
      window.removeEventListener('message', messageHandler);
      clearInterval(pollInterval);
      if (data.code) {
        resolve(data.code);
      } else {
        reject(new Error(data.error ?? m.settings_smtp_auth_oauth_error()));
      }
    };

    const pollInterval = setInterval(() => {
      if (popup.closed) {
        window.removeEventListener('message', messageHandler);
        clearInterval(pollInterval);
        reject(new Error(m.settings_smtp_auth_oauth_popup_closed()));
      }
    }, 500);

    window.addEventListener('message', messageHandler);
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
