import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import parse from 'html-react-parser';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { LabeledCheckbox } from '../../../../../shared/defguard-ui/components/Layout/LabeledCheckbox/LabeledCheckbox';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../shared/mutations';
import { QueryKeys } from '../../../../../shared/queries';
import { invalidateMultipleQueries } from '../../../../../shared/utils/invalidateMultipleQueries';
import { useSettingsPage } from '../../../hooks/useSettingsPage';

export const OpenIdGeneralSettings = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.openIdSettings;
  const toaster = useToaster();
  const {
    settings: { patchSettings },
  } = useApi();

  const settings = useSettingsPage((state) => state.settings);
  const enterpriseEnabled = useAppStore((s) => s.appInfo?.license_info.enterprise);

  const queryClient = useQueryClient();

  const { mutate, isPending: isLoading } = useMutation({
    mutationKey: [MutationKeys.EDIT_SETTINGS],
    mutationFn: patchSettings,
    onSuccess: () => {
      invalidateMultipleQueries(queryClient, [
        [QueryKeys.FETCH_SETTINGS],
        [QueryKeys.FETCH_ESSENTIAL_SETTINGS],
      ]);
      toaster.success(LL.settingsPage.messages.editSuccess());
    },
    onError: () => {
      toaster.error(LL.messages.error());
    },
  });

  if (!settings) return null;

  return (
    <section id="openid-settings">
      <header>
        <h2>{localLL.general.title()}</h2>
        <Helper>{parse(localLL.general.helper())}</Helper>
      </header>
      <div>
        <div>
          <div className="checkbox-row">
            <LabeledCheckbox
              disabled={isLoading || !enterpriseEnabled}
              label={localLL.general.createAccount.label()}
              value={settings.openid_create_account}
              onChange={() =>
                mutate({ openid_create_account: !settings.openid_create_account })
              }
            />
            <Helper>{localLL.general.createAccount.helper()}</Helper>
          </div>
        </div>
      </div>
    </section>
  );
};
