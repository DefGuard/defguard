import './styles.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import parse from 'html-react-parser';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { Helper } from '../../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { LabeledCheckbox } from '../../../../../../shared/defguard-ui/components/Layout/LabeledCheckbox/LabeledCheckbox';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { externalLink } from '../../../../../../shared/links';
import { MutationKeys } from '../../../../../../shared/mutations';
import { QueryKeys } from '../../../../../../shared/queries';
import { invalidateMultipleQueries } from '../../../../../../shared/utils/invalidateMultipleQueries';
import { useSettingsPage } from '../../../../hooks/useSettingsPage';

export const ModulesSettings = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    settings: { patchSettings },
  } = useApi();

  const settings = useSettingsPage((state) => state.settings);

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
    <section id="modules-settings">
      <header>
        <h2>{LL.settingsPage.modulesVisibility.header()}</h2>
        <Helper>
          {parse(
            LL.settingsPage.modulesVisibility.helper({
              documentationLink: externalLink.gitbook.base,
            }),
          )}
        </Helper>
      </header>
      <Card shaded bordered hideMobile>
        <LabeledCheckbox
          disabled={isLoading}
          label={LL.settingsPage.modulesVisibility.fields.openid_enabled.label()}
          value={settings.openid_enabled}
          onChange={() => mutate({ openid_enabled: !settings.openid_enabled })}
        />
        <LabeledCheckbox
          label={LL.settingsPage.modulesVisibility.fields.wireguard_enabled.label()}
          value={settings.wireguard_enabled}
          disabled={isLoading}
          onChange={() => mutate({ wireguard_enabled: !settings.wireguard_enabled })}
        />
        <LabeledCheckbox
          label={LL.settingsPage.modulesVisibility.fields.worker_enabled.label()}
          value={settings.worker_enabled}
          disabled={isLoading}
          onChange={() => mutate({ worker_enabled: !settings.worker_enabled })}
        />
        <LabeledCheckbox
          label={LL.settingsPage.modulesVisibility.fields.webhooks_enabled.label()}
          value={settings.webhooks_enabled}
          disabled={isLoading}
          onChange={() => mutate({ webhooks_enabled: !settings.webhooks_enabled })}
        />
      </Card>
    </section>
  );
};
