import { useQuery } from '@tanstack/react-query';
import { useMatch, useNavigate } from '@tanstack/react-router';
import { type PropsWithChildren, useEffect } from 'react';
import { useAuth } from '../hooks/useAuth';
import { userMeQueryOptions } from '../query';

export const AppAuthProvider = ({ children }: PropsWithChildren) => {
  const navigate = useNavigate();
  const authMatch = useMatch({ from: '/auth/', shouldThrow: false });

  const setUser = useAuth((s) => s.setUser);
  const mfa = useAuth((s) => s.mfaLogin);

  const { data: response, isError, isLoading } = useQuery(userMeQueryOptions);

  useEffect(() => {
    if (mfa && mfa.mfa_method !== 'none' && authMatch) {
      switch (mfa.mfa_method) {
        case 'OneTimePassword':
          navigate({
            to: '/auth/mfa/totp',
            replace: true,
          });
          break;
        default:
          throw new Error('Unimplemented Factor');
      }
    }
  }, [mfa, authMatch, navigate]);

  useEffect(() => {
    if (isError && !isLoading && !authMatch) {
      setUser();
      navigate({
        to: '/auth/login',
        replace: true,
      });
    }
    if (!isLoading && response) {
      setUser(response.data);
      navigate({
        to: '/user/$username',
        params: {
          username: response.data.username,
        },
      });
    }
  }, [authMatch, isError, isLoading, navigate, response, setUser]);

  return <>{children}</>;
};
