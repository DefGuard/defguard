import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { LabeledCheckbox } from '../../../../../shared/defguard-ui/components/Layout/LabeledCheckbox/LabeledCheckbox';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../shared/mutations';
import { QueryKeys } from '../../../../../shared/queries';
import { useSettingsPage } from '../../../hooks/useSettingsPage';

export const OpenIdGeneralSettings = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.openIdSettings;
  const toaster = useToaster();
  const {
    settings: { patchSettings },
  } = useApi();

  const settings = useSettingsPage((state) => state.settings);

  const queryClient = useQueryClient();

  const { mutate, isLoading } = useMutation([MutationKeys.EDIT_SETTINGS], patchSettings, {
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_ESSENTIAL_SETTINGS]);
      queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
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
        <h2>{localLL.titleGeneral()}</h2>
      </header>
      <div>
        <LabeledCheckbox
          disabled={isLoading}
          label={localLL.general.createAccount()}
          value={settings.openid_create_account}
          onChange={() =>
            mutate({ openid_create_account: !settings.openid_create_account })
          }
        />
      </div>
    </section>
  );
};
