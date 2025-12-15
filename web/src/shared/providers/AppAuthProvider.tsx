import { useQuery } from '@tanstack/react-query';
import { useNavigate, useRouterState } from '@tanstack/react-router';
import type { AxiosError } from 'axios';
import { type PropsWithChildren, useEffect, useMemo } from 'react';
import { isPresent } from '../defguard-ui/utils/isPresent';
import { useAuth } from '../hooks/useAuth';
import { userMeQueryOptions } from '../query';

export const AppAuthProvider = ({ children }: PropsWithChildren) => {
  const navigate = useNavigate();
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const authMatch = useMemo(() => pathname.startsWith('/auth'), [pathname]);

  const setUser = useAuth((s) => s.setUser);
  const mfa = useAuth((s) => s.mfaLogin);
  const storedUser = useAuth((s) => s.user);
  const openIdConsentData = useAuth((s) => s.consentData);

  const { data: response, error } = useQuery(userMeQueryOptions);

  // if me endpoint fails with 401 then auth store should reset and that should send user back to login
  useEffect(() => {
    if (isPresent(error)) {
      const e = error as AxiosError;
      const status = e.status;
      if (isPresent(status) && status === 401) {
        setUser();
      }
    }
  }, [error, setUser]);

  // when login requires internal 2FA this handles automatic redirect
  useEffect(() => {
    if (mfa && mfa.mfa_method !== 'none' && authMatch && !storedUser) {
      switch (mfa.mfa_method) {
        case 'OneTimePassword':
          navigate({
            to: '/auth/mfa/totp',
            replace: true,
          });
          break;
        case 'Webauthn':
          navigate({
            to: '/auth/mfa/webauthn',
            replace: true,
          });
          break;
        case 'Email':
          navigate({
            to: '/auth/mfa/email',
            replace: true,
          });
          break;
        default:
          throw new Error('Unimplemented Factor');
      }
    }
  }, [mfa, authMatch, navigate, storedUser]);

  // store response in store
  useEffect(() => {
    if (response) {
      setUser(response.data);
    }
  }, [response, setUser]);

  // handle automatic redirects when auth store changes
  // biome-ignore lint/correctness/useExhaustiveDependencies: Only check storedUser and authMatch
  useEffect(() => {
    if (storedUser && authMatch) {
      if (isPresent(openIdConsentData)) {
        navigate({
          to: '/consent',
          //@ts-expect-error
          search: openIdConsentData,
        }).then(() => {
          // clear this key so it doesn't cause another redirect
          useAuth.setState({
            consentData: undefined,
          });
        });
      } else {
        navigate({
          to: '/user/$username',
          params: {
            username: storedUser.username,
          },
        });
      }
    }
    if (!storedUser && !authMatch) {
      navigate({
        to: '/auth/login',
        replace: true,
      });
    }
  }, [authMatch, navigate, storedUser]);

  return <>{children}</>;
};
