import { MutationCache, QueryClient, type QueryKey } from '@tanstack/react-query';

type InvalidateMeta = { invalidate?: QueryKey[] | QueryKey };

let queryClient: QueryClient;

type RO = readonly unknown[];

const isArrayFlat = (arr: RO | readonly RO[]): boolean =>
  arr.every((item) => !Array.isArray(item));

const mutationCache = new MutationCache({
  onSuccess: async (_data, _variables, _context, mutation) => {
    const keys = (mutation.meta as InvalidateMeta | undefined)?.invalidate;
    if (!Array.isArray(keys) || keys.length === 0) return;
    if (isArrayFlat(keys)) {
      await queryClient.invalidateQueries({ queryKey: keys });
    } else {
      await Promise.all(
        keys.map((key) => queryClient.invalidateQueries({ queryKey: key as QueryKey })),
      );
    }
  },
});

queryClient = new QueryClient({
  mutationCache,
  defaultOptions: {
    queries: {
      staleTime: Infinity,
      refetchOnWindowFocus: false,
      refetchOnReconnect: false,
      retry: false,
      // @ts-expect-error
      placeholderData: (perv) => perv,
    },
  },
});

export { queryClient };
