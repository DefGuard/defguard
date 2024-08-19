import './styles.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import parse from 'html-react-parser';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { Card } from '../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { LabeledCheckbox } from '../../../../../shared/defguard-ui/components/Layout/LabeledCheckbox/LabeledCheckbox';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../shared/mutations';
import { QueryKeys } from '../../../../../shared/queries';
import { useSettingsPage } from '../../../hooks/useSettingsPage';

export const PermissionsForm = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    settings: { patchSettings },
  } = useApi();

  const settings = useSettingsPage((state) => state.settings);

  const queryClient = useQueryClient();

  const { mutate, isLoading } = useMutation([MutationKeys.EDIT_SETTINGS], patchSettings, {
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
      toaster.success(LL.settingsPage.messages.editSuccess());
    },
    onError: (err: AxiosError) => {
      toaster.error(LL.messages.error());
      console.error(err);
    },
  });

  if (!settings) return null;

  return (
    <section id="permissions-settings">
      <header>
        <h2>{LL.settingsPage.permissions.header()}</h2>
        <Helper>{parse(LL.settingsPage.permissions.helper())}</Helper>
      </header>
      <Card shaded bordered hideMobile>
        <LabeledCheckbox
          disabled={isLoading}
          label={LL.settingsPage.permissions.fields.deviceCreation.label()}
          value={settings.disable_device_creation}
          onChange={() =>
            mutate({ disable_device_creation: !settings.disable_device_creation })
          }
        />
      </Card>
    </section>
  );
};
