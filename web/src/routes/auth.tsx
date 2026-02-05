import { useQueryClient } from '@tanstack/react-query';
import { createFileRoute, Outlet, useNavigate } from '@tanstack/react-router';
import { useCallback, useEffect } from 'react';
import z from 'zod';
import { type User, UserMfaMethod } from '../shared/api/types';
import { isPresent } from '../shared/defguard-ui/utils/isPresent';
import { useAuth } from '../shared/hooks/useAuth';

const basicSchema = z.object({
  url: z.string().nullable().optional(),
  user: z.custom<User>().nonoptional(),
});

const mfaSchema = z.object({
  mfa_method: z.enum(UserMfaMethod),
  totp_available: z.boolean(),
  webauthn_available: z.boolean(),
  email_available: z.boolean(),
});

export const Route = createFileRoute('/auth')({
  component: RouteComponent,
});

function RouteComponent() {
  const loginSubject = useAuth((s) => s.authSubject);
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const navigateToAuthorized = useCallback(
    (user: User) => {
      if (user.is_admin) {
        navigate({ to: '/vpn-overview' });
      } else {
        navigate({
          to: '/user/$username',
          params: {
            username: user.username,
          },
          replace: true,
        });
      }
    },
    [navigate],
  );

  // biome-ignore lint/correctness/useExhaustiveDependencies: rxjs sub
  useEffect(() => {
    const sub = loginSubject.subscribe((state) => {
      const basicResult = basicSchema.safeParse(state);
      const basicResponse = basicResult.data;
      if (isPresent(basicResponse) && basicResult.success) {
        void queryClient.invalidateQueries({
          queryKey: ['me'],
        });
        useAuth.getState().setUser(basicResponse.user);
        if (isPresent(basicResponse.url)) {
          window.location.replace(basicResponse.url);
          return;
        }
        setTimeout(() => {
          if (isPresent(useAuth.getState().consentData)) {
            //@ts-expect-error
            navigate({ to: '/consent', search: useAuth.getState().consentData });
          } else {
            navigateToAuthorized(basicResponse.user);
          }
        }, 200);
      } else {
        const mfaSchemaResult = mfaSchema.safeParse(state);
        const mfaResponse = mfaSchemaResult.data;
        if (isPresent(mfaResponse) && mfaSchemaResult.success) {
          useAuth.setState({ mfaLogin: mfaResponse });
          switch (mfaResponse.mfa_method) {
            case 'none':
              console.error('Cannot login with MFA on a user with no MFA set');
              break;
            case 'OneTimePassword':
              navigate({ to: '/auth/mfa/totp', replace: true });
              break;
            case 'Email':
              navigate({ to: '/auth/mfa/email', replace: true });
              break;
            case 'Webauthn':
              navigate({ to: '/auth/mfa/webauthn', replace: true });
              break;
          }
        } else {
          console.error('Unknown response schema for login');
        }
      }
    });
    return () => {
      sub.unsubscribe();
    };
  }, [loginSubject]);

  return <Outlet />;
}
