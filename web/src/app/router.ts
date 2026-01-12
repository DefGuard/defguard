import { createRouter } from '@tanstack/react-router';
import { routeTree } from '../routeTree.gen';
import { DefaultNotFound } from './DefaultNotFound';
import { queryClient } from './query';

export const router = createRouter({
  routeTree,
  defaultPreloadStaleTime: 0,
  defaultNotFoundComponent: DefaultNotFound,
  context: {
    queryClient,
  },
});

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
