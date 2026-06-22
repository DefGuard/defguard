import { createFileRoute } from '@tanstack/react-router';
import { useEffect } from 'react';

export const SMTP_OAUTH_CALLBACK_TYPE = 'smtp-oauth-callback';
// Fallback key for when COOP severs window.opener (see oauthFlow.ts).
export const SMTP_OAUTH_RESULT_KEY = 'smtp_oauth_result';

export const Route = createFileRoute('/smtp-oauth-callback')({
  component: SmtpOAuthCallbackPage,
});

function SmtpOAuthCallbackPage() {
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const code = params.get('code');
    const error = params.get('error');
    const errorDescription = params.get('error_description');

    const result = {
      type: SMTP_OAUTH_CALLBACK_TYPE,
      code: code ?? undefined,
      error: error ? (errorDescription ?? error) : undefined,
    };

    if (window.opener) {
      (window.opener as Window).postMessage(result, window.location.origin);
    } else {
      // COOP severed window.opener; storage event crosses browsing-context-group boundaries.
      // Do not persist OAuth authorization code in browser storage.
      const storageSafeResult = {
        type: SMTP_OAUTH_CALLBACK_TYPE,
        error: error ? (errorDescription ?? error) : undefined,
      };
      localStorage.setItem(SMTP_OAUTH_RESULT_KEY, JSON.stringify(storageSafeResult));
    }

    window.close();
  }, []);

  return null;
}
