import { queryOptions } from '@tanstack/react-query';
import api from './api/api';

export const userMeQueryOptions = queryOptions({
  queryFn: () => api.user.getMe.callbackFn(),
  queryKey: ['me'],
  staleTime: 60_000,
  throwOnError: false,
  retry: false,
  refetchOnWindowFocus: false,
  refetchOnMount: true,
  refetchOnReconnect: true,
});
