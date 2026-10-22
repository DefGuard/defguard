import './style.scss';
import { useMutation } from '@tanstack/react-query';
import type { AxiosError } from 'axios';
import { useEffect, useRef, useState } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import type { ApiError } from '../../../shared/api/types';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Checkbox } from '../../../shared/defguard-ui/components/Checkbox/Checkbox';
import { CodeCard } from '../../../shared/defguard-ui/components/CodeCard/CodeCard';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { Helper } from '../../../shared/defguard-ui/components/Helper/Helper';
import { Icon, IconKind } from '../../../shared/defguard-ui/components/Icon';
import { LoaderSpinner } from '../../../shared/defguard-ui/components/LoaderSpinner/LoaderSpinner';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing, ThemeVariable } from '../../../shared/defguard-ui/types';
import { useMigrationWizardStore } from '../store/useMigrationWizardStore';

export const MigrationWizardLdapMigrationStep = () => {
  const startedRef = useRef(false);
  const [disableSync, setDisableSync] = useState(false);
  const repair = useMutation({ mutationFn: api.migration.state.repairLdap });
  const disable = useMutation({
    mutationFn: () => api.settings.patchSettings({ ldap_sync_enabled: false }),
    onSuccess: () => useMigrationWizardStore.getState().next(),
  });
  const isPending = repair.isPending || disable.isPending;
  const disableError = disable.isError
    ? ((disable.error as AxiosError<ApiError>).response?.data?.msg ??
      m.migration_wizard_ldap_disable_sync_error())
    : null;
  const logValue = repair.isError
    ? ((
        repair.error as AxiosError<ApiError & { logs?: string[] }> | null
      )?.response?.data?.logs?.join('\n') ?? '')
    : (repair.data?.data.logs.join('\n') ?? '');
  const statusTitle = repair.isSuccess
    ? m.migration_wizard_ldap_status_success()
    : repair.isError
      ? m.migration_wizard_ldap_status_error()
      : m.migration_wizard_ldap_status_running();

  useEffect(() => {
    if (startedRef.current) return;
    startedRef.current = true;
    repair.mutate();
  }, [repair.mutate]);

  return (
    <WizardCard>
      <div className="ldap-migration-step">
        <div className="status-heading">
          {repair.isPending && <LoaderSpinner size={20} variant="primary" />}
          {repair.isSuccess && (
            <Icon
              icon={IconKind.CheckCircle}
              staticColor={ThemeVariable.FgSuccess}
              size={20}
            />
          )}
          {repair.isError && (
            <Icon
              icon={IconKind.WarningFilled}
              staticColor={ThemeVariable.FgCritical}
              size={20}
            />
          )}
          <p>{statusTitle}</p>
        </div>
        {repair.isError && (
          <>
            <SizedBox height={ThemeSpacing.Xl} />
            <CodeCard
              title={m.migration_wizard_ldap_error_log_title()}
              value={logValue}
              copy
              download
            />
            <SizedBox height={ThemeSpacing.Xl} />
            <Button
              text={m.migration_wizard_ldap_retry()}
              onClick={() => {
                disable.reset();
                setDisableSync(false);
                repair.mutate();
              }}
              disabled={isPending}
            />
            <SizedBox height={ThemeSpacing.Xl3} />
            <Checkbox
              active={disableSync}
              disabled={isPending}
              onClick={() => setDisableSync((active) => !active)}
              text={m.migration_wizard_ldap_disable_sync_label()}
              helperBlock={
                <Helper icon="info-outlined">
                  {m.migration_wizard_ldap_disable_sync_helper()}
                </Helper>
              }
            />
          </>
        )}
      </div>
      {repair.isSuccess && (
        <>
          <SizedBox height={ThemeSpacing.Xl} />
          <CodeCard
            title={m.migration_wizard_ldap_success_log_title()}
            value={logValue}
            copy
            download
          />
          <SizedBox height={ThemeSpacing.Xl} />
        </>
      )}
      {disableError && <p role="alert">{disableError}</p>}
      <Divider spacing={ThemeSpacing.Xl2} />
      <Controls>
        <Button
          variant="outlined"
          text={m.controls_back()}
          onClick={() => useMigrationWizardStore.getState().back()}
          disabled={isPending}
        />
        <div className="right">
          <Button
            text={m.controls_continue()}
            onClick={() => {
              if (repair.isSuccess) useMigrationWizardStore.getState().next();
              else if (disableSync && !isPending) disable.mutate();
            }}
            loading={disable.isPending}
            disabled={repair.isPending || (!repair.isSuccess && !disableSync)}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
