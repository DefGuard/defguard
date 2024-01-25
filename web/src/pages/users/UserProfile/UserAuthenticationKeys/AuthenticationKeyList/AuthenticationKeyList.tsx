import './style.scss';

import { useQuery } from '@tanstack/react-query';

import useApi from '../../../../../shared/hooks/useApi';
import { QueryKeys } from '../../../../../shared/queries';
import { AuthenticationKeyCard } from '../AuthenticationKeyCard/AuthenticationKeyCard';

export const AuthenticationKeyList = () => {
  const {
    user: { fetchAuthenticationKeys },
  } = useApi();

  const { data: authenticationKeys } = useQuery({
    queryFn: fetchAuthenticationKeys,
    queryKey: [QueryKeys.FETCH_AUTHENTICATION_KEYS],
    refetchOnMount: true,
    refetchOnWindowFocus: false,
  });

  return (
    <div className="authentication-key-list">
      {authenticationKeys?.map((item) => {
        return <AuthenticationKeyCard key={item.id} authentication_key={item} />;
      })}
    </div>
  );
};
