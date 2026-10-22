import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useMigrationWizardStore } from '../src/pages/MigrationWizardPage/store/useMigrationWizardStore';
import { MigrationWizardStep } from '../src/pages/MigrationWizardPage/types';
import api from '../src/shared/api/api';

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
          updateMigrationState: vi.fn().mockResolvedValue({}),
        },
      },
    },
  };
});

describe('migration wizard LDAP navigation', () => {
  beforeEach(() => {
    useMigrationWizardStore.getState().resetState();
    vi.clearAllMocks();
  });

  it('skips LDAP when migration is not required', () => {
    useMigrationWizardStore.setState({ current_step: MigrationWizardStep.General });

    useMigrationWizardStore.getState().next();

    expect(useMigrationWizardStore.getState().current_step).toBe(MigrationWizardStep.Ca);
  });

  it('navigates General to LDAP to CA and back when required', () => {
    useMigrationWizardStore.setState({
      current_step: MigrationWizardStep.General,
      ldap_migration_required: true,
    });

    useMigrationWizardStore.getState().next();
    expect(useMigrationWizardStore.getState().current_step).toBe(
      MigrationWizardStep.LdapMigration,
    );
    useMigrationWizardStore.getState().next();
    expect(useMigrationWizardStore.getState().current_step).toBe(MigrationWizardStep.Ca);
    useMigrationWizardStore.getState().back();
    expect(useMigrationWizardStore.getState().current_step).toBe(
      MigrationWizardStep.LdapMigration,
    );
    useMigrationWizardStore.getState().back();
    expect(useMigrationWizardStore.getState().current_step).toBe(
      MigrationWizardStep.General,
    );
    expect(api.migration.state.updateMigrationState).toHaveBeenCalled();
  });
});
