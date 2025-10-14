import { MutationCache, QueryClient, type QueryKey } from '@tanstack/react-query';

type InvalidateMeta = { invalidate?: QueryKey[] };

let queryClient: QueryClient;

const mutationCache = new MutationCache({
  onSuccess: async (_data, _variables, _context, mutation) => {
    const keys = (mutation.meta as InvalidateMeta | undefined)?.invalidate;
    if (!keys?.length) return;
    await Promise.all(
      keys.map((key) => queryClient.invalidateQueries({ queryKey: key })),
    );
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
    },
  },
});

export { queryClient };
