import './styles.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import parse from 'html-react-parser';
import { cloneDeep } from 'lodash-es';

import { useI18nContext } from '../../../i18n/i18n-react';
import { Card } from '../../../shared/components/layout/Card/Card';
import { CheckBox } from '../../../shared/components/layout/Checkbox/CheckBox';
import { Helper } from '../../../shared/components/layout/Helper/Helper';
import { useAppStore } from '../../../shared/hooks/store/useAppStore';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { externalLink } from '../../../shared/links';
import { MutationKeys } from '../../../shared/mutations';
import { QueryKeys } from '../../../shared/queries';
import { Settings } from '../../../shared/types';

type ModulesSettings =
  | 'openid_enabled'
  | 'ldap_enabled'
  | 'wireguard_enabled'
  | 'webhooks_enabled'
  | 'worker_enabled';

export const ModulesCard = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    settings: { editSettings },
  } = useApi();

  const settings = useAppStore((state) => state.settings);

  const queryClient = useQueryClient();

  const { mutate, isLoading } = useMutation([MutationKeys.EDIT_SETTINGS], editSettings, {
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
      toaster.success(LL.settingsPage.messages.editSuccess());
    },
    onError: () => {
      toaster.error(LL.messages.error());
    },
  });

  const handleChange = (key: ModulesSettings) => {
    if (settings && !isLoading) {
      const cloned = cloneDeep(settings) as Settings;
      cloned[key] = !cloned[key];
      mutate(cloned);
    }
  };

  if (!settings) return null;

  return (
    <section className="modules">
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
      <Card>
        <CheckBox
          disabled={isLoading}
          label={LL.settingsPage.modulesVisibility.fields.openid_enabled.label()}
          value={settings.openid_enabled}
          onChange={() => handleChange('openid_enabled')}
        />
        <CheckBox
          label={LL.settingsPage.modulesVisibility.fields.wireguard_enabled.label()}
          value={settings.wireguard_enabled}
          disabled={isLoading}
          onChange={() => handleChange('wireguard_enabled')}
        />
        <CheckBox
          label={LL.settingsPage.modulesVisibility.fields.worker_enabled.label()}
          value={settings.worker_enabled}
          disabled={isLoading}
          onChange={() => handleChange('worker_enabled')}
        />
        <CheckBox
          label={LL.settingsPage.modulesVisibility.fields.webhooks_enabled.label()}
          value={settings.webhooks_enabled}
          disabled={isLoading}
          onChange={() => handleChange('webhooks_enabled')}
        />
      </Card>
    </section>
  );
};
