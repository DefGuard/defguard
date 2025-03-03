import { useQuery } from '@tanstack/react-query';
import { PropsWithChildren, useEffect, useMemo } from 'react';
import { useLocation } from 'react-router';
import { useSearchParams } from 'react-router-dom';

import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import { useAclCreateSelector, useUpdateAclCreateContext } from './acl-context';

type Props = PropsWithChildren;
export const AclCreateDataProvider = ({ children }: Props) => {
  const location = useLocation();
  const [searchParams] = useSearchParams();
  const updateContext = useUpdateAclCreateContext();
  const baseContextSet = useAclCreateSelector(
    (s) => ![s.devices, s.groups, s.networks, s.users].includes(undefined),
  );
  const ruleEditSet = useAclCreateSelector((s) => isPresent(s.editRule));

  const {
    standaloneDevice: { getDevicesList },
    groups: { getGroupsInfo },
    user: { getUsers },
    network: { getNetworks },
    acl: {
      rules: { getRule },
    },
  } = useApi();

  const isRuleEdit = useMemo(
    () => location.pathname.includes('/acl/form') && location.search.includes('edit=1'),
    [location.pathname, location.search],
  );

  const editRuleId = useMemo(() => {
    if (isRuleEdit) {
      return parseInt(searchParams.get('rule') as string);
    }
  }, [isRuleEdit, searchParams]);

  const { data: editRuleData } = useQuery({
    queryFn: () => getRule(editRuleId as number),
    queryKey: [QueryKeys.FETCH_ACL_RULE_EDIT, editRuleId],
    enabled: isRuleEdit && isPresent(editRuleId),
    refetchOnMount: true,
    refetchOnWindowFocus: false,
  });

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

  const contextSet = useMemo(() => {
    if (isRuleEdit) {
      return baseContextSet && ruleEditSet;
    }
    return baseContextSet;
  }, [baseContextSet, isRuleEdit, ruleEditSet]);

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

  useEffect(() => {
    updateContext({
      editRule: editRuleData,
    });
  }, [editRuleData, updateContext]);

  if (!contextSet) return null;

  return <>{children}</>;
};
