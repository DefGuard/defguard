import { queryOptions } from '@tanstack/react-query';
import api from './api/api';
import type { UserProfile } from './api/types';
import { updateServiceApi } from './api/update-service';

export const userMeQueryOptions = queryOptions({
  queryFn: () => api.user.getMe,
  queryKey: ['me'],
  staleTime: 60_000,
  throwOnError: false,
  retry: false,
  refetchOnWindowFocus: false,
  refetchOnMount: true,
  refetchOnReconnect: true,
});

export const userProfileQueryOptions = (username: string) =>
  queryOptions({
    queryFn: () => api.user.getUser(username),
    select: ({ data }) => {
      const res: UserProfile = {
        devices: data.devices.map((device) => ({
          ...device,
          biometry_enabled: data.biometric_enabled_devices.includes(device.id),
        })),
        security_keys: data.security_keys,
        user: data.user,
      };
      return res;
    },
    queryKey: ['user', username],
    refetchOnMount: true,
    refetchOnReconnect: true,
  });

export const clientArtifactsQueryOptions = queryOptions({
  queryFn: updateServiceApi.getClientArtifacts,
  queryKey: ['update-service', 'artifacts'],
  staleTime: 180 * 1000,
  refetchOnWindowFocus: false,
  refetchOnMount: true,
  refetchOnReconnect: true,
});

export const getUserAuthKeysQueryOptions = (username: string) =>
  queryOptions({
    queryFn: () => api.user.getAuthKeys(username),
    queryKey: ['user', username, 'auth_key'],
    select: (response) => response.data,
    refetchOnMount: true,
    refetchOnReconnect: true,
  });

export const getUserApiTokensQueryOptions = (username: string) =>
  queryOptions({
    queryFn: () => api.user.getApiTokens(username),
    queryKey: ['user', username, 'api_token'],
    select: (resp) => resp.data,
    refetchOnMount: true,
    refetchOnReconnect: true,
  });

export const getUsersQueryOptions = queryOptions({
  queryFn: api.getUsersOverview,
  queryKey: ['user'],
  refetchOnMount: true,
  refetchOnReconnect: true,
});

export const getGroupsInfoQueryOptions = queryOptions({
  queryFn: api.group.getGroupsInfo,
  queryKey: ['group-info'],
  select: (resp) => resp.data,
  refetchOnMount: true,
  refetchOnReconnect: true,
});
