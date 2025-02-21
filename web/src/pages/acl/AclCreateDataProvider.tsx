import { useQuery } from '@tanstack/react-query';
import { PropsWithChildren, useEffect } from 'react';

import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import { useAclCreateSelector, useUpdateAclCreateContext } from './acl-context';

type Props = PropsWithChildren;
export const AclCreateDataProvider = ({ children }: Props) => {
  const updateContext = useUpdateAclCreateContext();
  const contextSet = useAclCreateSelector(
    (s) => ![s.devices, s.groups, s.networks, s.users].includes(undefined),
  );

  const {
    standaloneDevice: { getDevicesList },
    groups: { getGroupsInfo },
    user: { getUsers },
    network: { getNetworks },
  } = useApi();

  const { data: aclData } = useQuery({
    queryKey: [
      QueryKeys.FETCH_ACL_CREATE_CONTEXT,
      QueryKeys.FETCH_USERS_LIST,
      QueryKeys.FETCH_GROUPS_INFO,
      QueryKeys.FETCH_NETWORKS,
      QueryKeys.FETCH_STANDALONE_DEVICE_LIST,
    ],
    queryFn: () =>
      Promise.all([getNetworks(), getGroupsInfo(), getUsers(), getDevicesList()]),
    refetchOnReconnect: true,
    refetchOnWindowFocus: false,
    refetchOnMount: true,
  });

  useEffect(() => {
    if (aclData) {
      const [networks, groups, users, devices] = aclData;
      updateContext({
        devices,
        groups,
        networks,
        users,
      });
    }
  }, [aclData, updateContext]);

  if (!contextSet) return null;

  return <>{children}</>;
};
