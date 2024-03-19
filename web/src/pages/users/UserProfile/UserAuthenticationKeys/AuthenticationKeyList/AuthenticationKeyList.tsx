import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { groupBy, isUndefined } from 'lodash-es';
import { Fragment, useMemo } from 'react';

import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../../shared/hooks/useApi';
import { QueryKeys } from '../../../../../shared/queries';
import { AuthenticationKey } from '../../../../../shared/types';
import { AuthenticationKeyItem } from './AuthenticationKeyItem/AuthenticationKeyItem';
import { AuthenticationKeyItemYubikey } from './AuthenticationKeyItemYubiKey/AuthenticationKeyItemYubiKey';

type itemData = {
  yubikey?: {
    yubikey_id: number;
    yubikey_name: string;
    yubikey_serial: string;
    keys: AuthenticationKey[];
  };
  key?: AuthenticationKey;
};

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

  // parse api response then store it and return in form which can be displayed by components
  const items = useMemo((): itemData[] => {
    if (authenticationKeysInfo) {
      const standAlone: itemData[] = authenticationKeysInfo
        .filter((k) => isUndefined(k.yubikey_id))
        .map((k) => ({
          key: {
            id: k.id,
            name: k.name as string,
            key_type: k.key_type,
            key: k.key,
          },
        }));
      const yubikeys: itemData[] = [];
      const g = groupBy(
        authenticationKeysInfo.filter((k) => !isUndefined(k.yubikey_id)),
        'yubikey_id',
      );
      Object.keys(g).forEach((string_id) => {
        const val = g[string_id];
        const keys: AuthenticationKey[] = val.map((info) => ({
          id: info.id,
          key: info.key,
          key_type: info.key_type,
          name: info.yubikey_name as string,
        }));
        yubikeys.push({
          yubikey: {
            keys,
            yubikey_id: val[0].yubikey_id as number,
            yubikey_name: val[0].yubikey_name as string,
            yubikey_serial: val[0].yubikey_serial as string,
          },
        });
      });

      // sort out by names desc
      const res = [...yubikeys, ...standAlone].sort((a, b) => {
        const nameA = a.yubikey?.yubikey_name || a.key?.name || '';
        const nameB = b.yubikey?.yubikey_name || b.key?.name || '';
        if (nameA > nameB) {
          return -1;
        }
        if (nameA < nameB) {
          return 1;
        }
        return 0;
      });

      return res;
    }
    return [];
  }, [authenticationKeysInfo]);

  if (items.length === 0 || !items) return null;

  return (
    <div className="authentication-key-list">
      {items.map((item, index) => (
        <Fragment key={(item?.key?.id || item?.yubikey?.yubikey_id) ?? index}>
          {item.yubikey && (
            <AuthenticationKeyItemYubikey
              yubikey={{
                yubikey_id: item.yubikey.yubikey_id,
                yubikey_name: item.yubikey.yubikey_name,
                yubikey_serial: item.yubikey.yubikey_serial,
              }}
              keys={item.yubikey.keys}
            />
          )}
          {item.key && <AuthenticationKeyItem keyData={item.key} />}
        </Fragment>
      ))}
    </div>
  );
};
