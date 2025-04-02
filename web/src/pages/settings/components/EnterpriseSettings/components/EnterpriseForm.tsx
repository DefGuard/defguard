import './styles.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import parse from 'html-react-parser';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { LabeledCheckbox } from '../../../../../shared/defguard-ui/components/Layout/LabeledCheckbox/LabeledCheckbox';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../shared/mutations';
import { QueryKeys } from '../../../../../shared/queries';

export const EnterpriseForm = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    settings: { patchEnterpriseSettings },
  } = useApi();

  const settings = useAppStore((state) => state.enterprise_settings);

  const queryClient = useQueryClient();

  const { mutate, isPending: isLoading } = useMutation({
    mutationKey: [MutationKeys.EDIT_SETTINGS],
    mutationFn: patchEnterpriseSettings,
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_ENTERPRISE_SETTINGS],
      });
      toaster.success(LL.settingsPage.messages.editSuccess());
    },
    onError: (err: AxiosError) => {
      toaster.error(LL.messages.error());
      console.error(err);
    },
  });

  if (!settings) return null;

  return (
    <section id="enterprise-settings">
      <header>
        <div className="helper-row">
          <h2>{LL.settingsPage.enterprise.header()}</h2>
          <Helper>{LL.settingsPage.enterprise.helper()}</Helper>
        </div>
      </header>
      <div className="checkbox-column">
        <div className="helper-row">
          <LabeledCheckbox
            disabled={isLoading}
            label={LL.settingsPage.enterprise.fields.deviceManagement.label()}
            value={settings.admin_device_management}
            onChange={() =>
              mutate({ admin_device_management: !settings.admin_device_management })
            }
          />
          <Helper>
            {parse(LL.settingsPage.enterprise.fields.deviceManagement.helper())}
          </Helper>
        </div>
        <div className="helper-row">
          <LabeledCheckbox
            disabled={isLoading}
            label={LL.settingsPage.enterprise.fields.manualConfig.label()}
            value={settings.only_client_activation}
            onChange={() =>
              mutate({ only_client_activation: !settings.only_client_activation })
            }
          />
          <Helper>
            {parse(LL.settingsPage.enterprise.fields.manualConfig.helper())}
          </Helper>
        </div>
        <div className="helper-row">
          <LabeledCheckbox
            disabled={isLoading}
            label={LL.settingsPage.enterprise.fields.disableAllTraffic.label()}
            value={settings.disable_all_traffic}
            onChange={() =>
              mutate({ disable_all_traffic: !settings.disable_all_traffic })
            }
          />
          <Helper>
            {parse(LL.settingsPage.enterprise.fields.disableAllTraffic.helper())}
          </Helper>
        </div>
      </div>
    </section>
  );
};
