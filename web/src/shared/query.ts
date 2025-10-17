import { queryOptions } from '@tanstack/react-query';
import api from './api/api';
import type { UserProfile } from './api/types';

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
