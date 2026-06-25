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

    const result = {
      type: SMTP_OAUTH_CALLBACK_TYPE,
      code: code ?? undefined,
      error: error ? (errorDescription ?? error) : undefined,
    };

    if (window.opener) {
      (window.opener as Window).postMessage(result, window.location.origin);
    } else {
      // COOP severed window.opener; BroadcastChannel crosses browsing-context-group
      // boundaries without touching browser storage.
      try {
        const channel = new BroadcastChannel('smtp-oauth-relay');
        channel.postMessage(result);
        channel.close();
      } catch {
        // ignore
      }
    }
    window.close();
  }, []);

  return null;
}
