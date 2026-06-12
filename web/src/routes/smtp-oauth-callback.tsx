import { createFileRoute } from '@tanstack/react-router';
import { useEffect } from 'react';

export const SMTP_OAUTH_CALLBACK_TYPE = 'smtp-oauth-callback';

export const Route = createFileRoute('/smtp-oauth-callback')({
  component: SmtpOAuthCallbackPage,
});

function SmtpOAuthCallbackPage() {
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const code = params.get('code');
    const error = params.get('error');
    const errorDescription = params.get('error_description');

    if (window.opener) {
      (window.opener as Window).postMessage(
        {
          type: SMTP_OAUTH_CALLBACK_TYPE,
          code: code ?? undefined,
          error: error ? (errorDescription ?? error) : undefined,
        },
        window.location.origin,
      );
    }

    window.close();
  }, []);

  return null;
}
