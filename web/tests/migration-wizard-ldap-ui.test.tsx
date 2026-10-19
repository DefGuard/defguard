import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { type ReactNode, StrictMode } from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { MigrationWizardLdapMigrationStep } from '../src/pages/MigrationWizardPage/steps/MigrationWizardLdapMigrationStep';
import { MigrationWizardStart } from '../src/pages/MigrationWizardPage/steps/MigrationWizardStart';
import { useMigrationWizardStore } from '../src/pages/MigrationWizardPage/store/useMigrationWizardStore';
import { MigrationWizardStep } from '../src/pages/MigrationWizardPage/types';

const apiMocks = vi.hoisted(() => ({
  getMigrationState: vi.fn(),
  repairLdap: vi.fn(),
  updateMigrationState: vi.fn().mockResolvedValue({}),
  patchSettings: vi.fn(),
}));

vi.mock('../src/shared/api/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../src/shared/api/api')>();
  return {
    ...actual,
    default: {
      ...actual.default,
      migration: {
        ...actual.default.migration,
        state: {
          ...actual.default.migration.state,
          getMigrationState: apiMocks.getMigrationState,
          repairLdap: apiMocks.repairLdap,
          updateMigrationState: apiMocks.updateMigrationState,
        },
      },
      settings: { ...actual.default.settings, patchSettings: apiMocks.patchSettings },
    },
  };
});

const deferred = <T,>() => {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
};

const renderWithClient = (element: ReactNode, strict = false) => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      {strict ? <StrictMode>{element}</StrictMode> : element}
    </QueryClientProvider>,
  );
};

const apiResponse = (data: unknown) => ({ data });

beforeEach(() => {
  useMigrationWizardStore.getState().resetState();
  apiMocks.getMigrationState.mockReset();
  apiMocks.repairLdap.mockReset();
  apiMocks.updateMigrationState.mockReset().mockResolvedValue({});
  apiMocks.patchSettings.mockReset();
});

describe('migration wizard LDAP UI', () => {
  it('refetches cached Start state, waits for success, and stays on welcome after failure', async () => {
    const queryClient = new QueryClient({
      defaultOptions: {
        queries: { staleTime: 30_000, retry: false },
        mutations: { retry: false },
      },
    });
    queryClient.setQueryData(
      ['migration', 'state'],
      apiResponse({ ldap_migration_required: false }),
    );
    useMigrationWizardStore.setState({ current_step: MigrationWizardStep.Welcome });
    const pendingCheck = deferred<ReturnType<typeof apiResponse>>();
    apiMocks.getMigrationState.mockReturnValueOnce(pendingCheck.promise);
    render(
      <QueryClientProvider client={queryClient}>
        <MigrationWizardStart />
      </QueryClientProvider>,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Start migration process' }));
    await waitFor(() => expect(apiMocks.getMigrationState).toHaveBeenCalledTimes(1));
    expect(
      screen.getByRole('button', { name: 'Start migration process' }),
    ).toBeDisabled();
    expect(
      screen
        .getByRole('button', { name: 'Start migration process' })
        .querySelector('.loader-overlay'),
    ).not.toBeNull();
    expect(useMigrationWizardStore.getState().current_step).toBe(
      MigrationWizardStep.Welcome,
    );
    pendingCheck.resolve(
      apiResponse({
        current_step: 'general',
        location_state: null,
        proxy_url: null,
        ldap_migration_required: true,
      }),
    );
    await waitFor(() =>
      expect(useMigrationWizardStore.getState().current_step).toBe(
        MigrationWizardStep.General,
      ),
    );
    expect(useMigrationWizardStore.getState().ldap_migration_required).toBe(true);

    cleanup();
    useMigrationWizardStore.getState().resetState();
    useMigrationWizardStore.setState({ current_step: MigrationWizardStep.Welcome });
    apiMocks.getMigrationState.mockRejectedValueOnce(new Error('offline'));
    renderWithClient(<MigrationWizardStart />);
    fireEvent.click(screen.getByRole('button', { name: 'Start migration process' }));
    await waitFor(() => expect(apiMocks.getMigrationState).toHaveBeenCalledTimes(2));
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Start migration process' }),
      ).toBeEnabled(),
    );
    expect(useMigrationWizardStore.getState().current_step).toBe(
      MigrationWizardStep.Welcome,
    );
  });

  it('runs repair once in StrictMode, shows success log, and enables Continue', async () => {
    const pendingRepair = deferred<ReturnType<typeof apiResponse>>();
    apiMocks.repairLdap.mockReturnValue(pendingRepair.promise);
    useMigrationWizardStore.setState({ current_step: MigrationWizardStep.LdapMigration });
    renderWithClient(<MigrationWizardLdapMigrationStep />, true);

    await waitFor(() => expect(apiMocks.repairLdap).toHaveBeenCalledTimes(1));
    expect(
      screen.getByText('Analyzing and migrating users. Please wait.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Continue' })).toBeDisabled();
    pendingRepair.resolve(apiResponse({ found: 3, migrated: 2 }));
    expect(await screen.findByText(/Accounts found: 3/)).toBeInTheDocument();
    expect(
      screen.getByText('LDAP user migration successfully completed.'),
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Continue' })).toBeEnabled();
  });

  it('shows repair error, retries, and displays success', async () => {
    apiMocks.repairLdap
      .mockRejectedValueOnce({ response: { data: { msg: 'repair failed' } } })
      .mockResolvedValueOnce(apiResponse({ found: 1, migrated: 1 }));
    useMigrationWizardStore.setState({ current_step: MigrationWizardStep.LdapMigration });
    const { container } = renderWithClient(<MigrationWizardLdapMigrationStep />);

    const errorLog = await screen.findByText('repair failed');
    const retryButton = screen.getByRole('button', { name: 'Retry' });
    expect(errorLog).toBeInTheDocument();
    expect(container.querySelector('.loading-step')).toBeNull();
    expect(errorLog.closest('.code-card')?.parentElement).toBe(retryButton.parentElement);
    expect(
      screen.getByText('An error occurred during the LDAP migration.'),
    ).toBeInTheDocument();
    fireEvent.click(retryButton);
    expect(await screen.findByText(/Accounts found: 1/)).toBeInTheDocument();
    expect(apiMocks.repairLdap).toHaveBeenCalledTimes(2);
  });

  it('does not advance for a failed skip PATCH and advances after retry succeeds', async () => {
    apiMocks.repairLdap.mockRejectedValue(new Error('repair failed'));
    const pendingPatch = deferred<unknown>();
    apiMocks.patchSettings
      .mockReturnValueOnce(pendingPatch.promise)
      .mockResolvedValueOnce({});
    useMigrationWizardStore.setState({ current_step: MigrationWizardStep.LdapMigration });
    renderWithClient(<MigrationWizardLdapMigrationStep />);
    await screen.findByText('An error occurred during the LDAP migration.');
    fireEvent.click(
      screen.getByRole('button', { name: /Disable LDAP synchronization and continue/ }),
    );

    fireEvent.click(screen.getByRole('button', { name: 'Continue' }));
    await waitFor(() => expect(apiMocks.patchSettings).toHaveBeenCalledTimes(1));
    expect(apiMocks.patchSettings).toHaveBeenCalledWith({ ldap_sync_enabled: false });
    expect(screen.getByRole('button', { name: 'Continue' })).toBeDisabled();
    expect(
      screen.getByRole('button', { name: 'Continue' }).querySelector('.loader-overlay'),
    ).not.toBeNull();
    pendingPatch.reject({ response: { data: { msg: 'settings failed' } } });
    expect(await screen.findByRole('alert')).toHaveTextContent('settings failed');
    expect(useMigrationWizardStore.getState().current_step).toBe(
      MigrationWizardStep.LdapMigration,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Continue' }));
    await waitFor(() =>
      expect(useMigrationWizardStore.getState().current_step).toBe(
        MigrationWizardStep.Ca,
      ),
    );
    expect(apiMocks.patchSettings).toHaveBeenCalledTimes(2);
  });
});
