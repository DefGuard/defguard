import './styles.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import parse from 'html-react-parser';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { Card } from '../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { LabeledCheckbox } from '../../../../../shared/defguard-ui/components/Layout/LabeledCheckbox/LabeledCheckbox';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../shared/mutations';
import { QueryKeys } from '../../../../../shared/queries';

export const BehaviourForm = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    settings: { patchEnterpriseSettings },
  } = useApi();

  const settings = useAppStore((state) => state.enterprise_settings);

  const queryClient = useQueryClient();

  const { mutate, isLoading } = useMutation(
    [MutationKeys.EDIT_SETTINGS],
    patchEnterpriseSettings,
    {
      onSuccess: () => {
        queryClient.invalidateQueries([QueryKeys.FETCH_ENTERPRISE_SETTINGS]);
        toaster.success(LL.settingsPage.messages.editSuccess());
      },
      onError: (err: AxiosError) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    },
  );

  if (!settings) return null;

  return (
    <section id="behaviour-settings">
      <header>
        <h2>{LL.settingsPage.behaviour.header()}</h2>
        <Helper>{parse(LL.settingsPage.behaviour.helper())}</Helper>
      </header>
      <Card shaded bordered hideMobile>
        <LabeledCheckbox
          disabled={isLoading}
          label={LL.settingsPage.behaviour.fields.deviceManagement.label()}
          value={settings.admin_device_management}
          onChange={() =>
            mutate({ admin_device_management: !settings.admin_device_management })
          }
        />
      </Card>
    </section>
  );
};
