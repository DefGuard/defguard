import { queryOptions } from '@tanstack/react-query';
import api from './api/api';
import type { UserProfile } from './api/types';
import { updateServiceApi } from './api/update-service';

export const getEnterpriseSettingsQueryOptions = queryOptions({
  queryFn: api.settings.getEnterpriseSettings,
  queryKey: ['settings_enterprise'],
  select: (resp) => resp.data,
});

export const getLocationQueryOptions = (id: number) =>
  queryOptions({
    queryFn: () => api.location.getLocation(id),
    queryKey: ['network', id],
    select: (resp) => resp.data,
  });

export const getLocationsQueryOptions = queryOptions({
  queryFn: api.location.getLocations,
  queryKey: ['network'],
  select: (resp) => resp.data,
});

export const getNetworkDevicesQueryOptions = queryOptions({
  queryFn: api.network_device.getDevices,
  queryKey: ['device', 'network'],
  select: (resp) => resp.data,
});

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

export const getUserApiTokensQueryOptions = (username: string, admin: boolean) =>
  queryOptions({
    queryFn: () => api.user.getApiTokens(username),
    queryKey: ['user', username, 'api_token'],
    select: (resp) => resp.data,
    refetchOnMount: true,
    refetchOnReconnect: true,
    throwOnError: false,
    enabled: admin,
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

export const getOpenIdClientQueryOptions = queryOptions({
  queryFn: api.openIdClient.getOpenIdClients,
  queryKey: ['oauth'],
  select: (resp) => resp.data,
});

export const getWebhooksQueryOptions = queryOptions({
  queryFn: api.webhook.getWebhooks,
  queryKey: ['webhook'],
  select: (resp) => resp.data,
});

export const getSettingsQueryOptions = queryOptions({
  queryFn: api.settings.getSettings,
  queryKey: ['settings'],
  select: (resp) => resp.data,
});

export const getOpenIdProvidersQueryOptions = queryOptions({
  queryFn: api.openIdProvider.getOpenIdProvider,
  queryKey: ['openid', 'provider'],
  select: (resp) => resp.data,
});
