import { QueryClient, QueryKey } from '@tanstack/query-core';

export const invalidateMultipleQueries = (
  client: QueryClient,
  keys: QueryKey[] | string[],
): void => {
  keys.forEach((k) => {
    if (Array.isArray(k)) {
      void client.invalidateQueries({
        queryKey: k,
      });
    } else {
      void client.invalidateQueries({
        queryKey: [k],
      });
    }
  });
};
