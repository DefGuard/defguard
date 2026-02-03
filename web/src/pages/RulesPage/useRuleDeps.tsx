import { useQuery } from '@tanstack/react-query';
import { objectify } from 'radashi';
import { useMemo } from 'react';
import type { Resource, ResourceById } from '../../shared/api/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import {
  getAliasesQueryOptions,
  getGroupsInfoQueryOptions,
  getLocationsQueryOptions,
  getNetworkDevicesQueryOptions,
  getUsersQueryOptions,
} from '../../shared/query';

const resourceById = <T extends Resource>(values?: T[]): ResourceById<T> | null =>
  isPresent(values)
    ? objectify(
        values,
        (item) => item.id,
        (item) => item,
      )
    : null;

export const useRuleDeps = () => {
  const { data: aliases, isLoading: aliasesLoading } = useQuery(getAliasesQueryOptions);
  const { data: groups, isLoading: groupsLoading } = useQuery(getGroupsInfoQueryOptions);
  const { data: locations, isLoading: locationsLoading } = useQuery(
    getLocationsQueryOptions,
  );
  const { data: users, isLoading: usersLoading } = useQuery(getUsersQueryOptions);
  const { data: devices, isLoading: devicesLoading } = useQuery(
    getNetworkDevicesQueryOptions,
  );

  const aliasesById = useMemo(() => resourceById(aliases), [aliases]);
  const groupsById = useMemo(() => resourceById(groups), [groups]);
  const locationsById = useMemo(() => resourceById(locations), [locations]);
  const usersById = useMemo(() => resourceById(users), [users]);
  const devicesById = useMemo(() => resourceById(devices), [devices]);

  return {
    loading:
      aliasesLoading ||
      groupsLoading ||
      locationsLoading ||
      usersLoading ||
      devicesLoading,
    aliases: aliasesById,
    groups: groupsById,
    locations: locationsById,
    users: usersById,
    devices: devicesById,
  };
};
