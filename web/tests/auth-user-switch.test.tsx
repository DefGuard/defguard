import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act, render } from '@testing-library/react';
import { type ComponentType, createElement } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Route } from '../src/routes/auth';
import api from '../src/shared/api/api';
import type { User } from '../src/shared/api/types';
import { useAuth } from '../src/shared/hooks/useAuth';
import { getLicenseInfoQueryOptions } from '../src/shared/query';

vi.mock('@tanstack/react-router', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@tanstack/react-router')>()),
  useNavigate: () => vi.fn(),
  Outlet: () => null,
}));

afterEach(() => {
  vi.clearAllTimers();
  vi.useRealTimers();
  vi.restoreAllMocks();
  useAuth.getState().reset();
});

describe('switching users', () => {
  it('discards the previous user license error when login succeeds', async () => {
    vi.useFakeTimers();
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const forbidden = new Error('Request failed with status code 403');
    vi.spyOn(api, 'getSessionInfo').mockResolvedValue({
      data: { authorized: true, is_admin: true, active_wizard: null, username: 'admin' },
    } as Awaited<ReturnType<typeof api.getSessionInfo>>);

    await expect(
      queryClient.fetchQuery({
        ...getLicenseInfoQueryOptions,
        queryFn: async () => {
          throw forbidden;
        },
      }),
    ).rejects.toThrow(forbidden);
    expect(queryClient.getQueryState(getLicenseInfoQueryOptions.queryKey)?.status).toBe(
      'error',
    );

    render(
      <QueryClientProvider client={queryClient}>
        {createElement(Route.options.component as ComponentType)}
      </QueryClientProvider>,
    );

    await act(async () => {
      useAuth.getState().authSubject.next({
        user: { username: 'admin', is_admin: true } as User,
      });
    });

    expect(
      queryClient.getQueryState(getLicenseInfoQueryOptions.queryKey),
    ).toBeUndefined();
    queryClient.clear();
  });
});
