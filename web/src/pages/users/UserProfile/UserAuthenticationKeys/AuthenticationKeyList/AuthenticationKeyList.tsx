import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';

import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../../shared/hooks/useApi';
import { QueryKeys } from '../../../../../shared/queries';
import { AuthenticationKey } from '../../../../../shared/types';
import { AuthenticationKeyCard } from '../AuthenticationKeyCard/AuthenticationKeyCard';

export const AuthenticationKeyList = () => {
  const user = useUserProfileStore((s) => s.userProfile?.user);
  const {
    user: { getAuthenticationKeysInfo: fetchAuthenticationKeys },
  } = useApi();

  const { data: authenticationKeysInfo } = useQuery({
    queryFn: () => fetchAuthenticationKeys({ username: user?.username as string }),
    queryKey: [QueryKeys.FETCH_AUTHENTICATION_KEYS_INFO, user?.username],
    refetchOnMount: true,
    refetchOnWindowFocus: false,
    enabled: !isUndefined(user),
  });

  const authenticationKeys = useMemo(() => {
    if (authenticationKeysInfo) {
      return authenticationKeysInfo
        .filter((i) => isUndefined(i.yubikey_id))
        .map((i) => {
          const res: AuthenticationKey = {
            id: i.id,
            key: i.key,
            key_type: i.key_type,
            name: i.name as string,
          };
          return res;
        });
    }
    return [];
  }, [authenticationKeysInfo]);

  return (
    <div className="authentication-key-list">
      {authenticationKeys.map((item) => {
        return <AuthenticationKeyCard key={item.id} authenticationKey={item} />;
      })}
    </div>
  );
};
