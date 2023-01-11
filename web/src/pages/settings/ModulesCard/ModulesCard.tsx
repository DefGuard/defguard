import './styles.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { cloneDeep } from 'lodash-es';
import { ReactNode } from 'react';

import Badge from '../../../shared/components/layout/Badge/Badge';
import { Card } from '../../../shared/components/layout/Card/Card';
import { CheckBox } from '../../../shared/components/layout/Checkbox/CheckBox';
import { Helper } from '../../../shared/components/layout/Helper/Helper';
import { useAppStore } from '../../../shared/hooks/store/useAppStore';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../shared/mutations';
import { QueryKeys } from '../../../shared/queries';
import { Settings } from '../../../shared/types';
import { useI18nContext } from '../../../i18n/i18n-react';
import parse from 'html-react-parser';

export const ModulesCard = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    settings: { editSettings },
  } = useApi();

  const settings = useAppStore((state) => state.settings);

  const queryClient = useQueryClient();

  const { mutate, isLoading } = useMutation(
    [MutationKeys.EDIT_SETTINGS],
    editSettings,
    {
      onSuccess: () => {
        queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
        toaster.success(LL.messages.success());
      },
      onError: () => {
        toaster.error(LL.messages.error());
      },
    }
  );

  const handleChange = (
    key: keyof Omit<
      Settings,
      | 'id'
      | 'challenge_template'
      | 'main_logo_url'
      | 'instance_name'
      | 'nav_logo_url'
    >
  ) => {
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
        <Helper>{parse(LL.settingsPage.modulesVisibility.helper())}</Helper>
      </header>
      <Card>
        <CheckBox
          label={LL.settingsPage.modulesVisibility.fields.wireguard_enabled.label()}
          value={settings.wireguard_enabled}
          disabled={isLoading}
          onChange={() => handleChange('wireguard_enabled')}
        />
        <CheckBox
          label={LL.settingsPage.modulesVisibility.fields.webhooks_enabled.label()}
          value={settings.webhooks_enabled}
          disabled={isLoading}
          onChange={() => handleChange('webhooks_enabled')}
        />
        <CheckBox
          label={LL.settingsPage.modulesVisibility.fields.web3_enabled.label()}
          value={settings.web3_enabled}
          disabled={isLoading}
          onChange={() => handleChange('web3_enabled')}
        />
        <CheckBox
          label={
            <EnterPriceLabel>
              {LL.settingsPage.modulesVisibility.fields.worker_enabled.label()}
            </EnterPriceLabel>
          }
          value={settings.worker_enabled}
          disabled={isLoading}
          onChange={() => handleChange('worker_enabled')}
        />
        <CheckBox
          disabled={isLoading}
          label={
            <EnterPriceLabel>
              {LL.settingsPage.modulesVisibility.fields.openid_enabled.label()}
            </EnterPriceLabel>
          }
          value={settings.openid_enabled}
          onChange={() => handleChange('openid_enabled')}
        />
        <CheckBox
          label={
            <EnterPriceLabel>
              {LL.settingsPage.modulesVisibility.fields.oauth_enabled.label()}
            </EnterPriceLabel>
          }
          value={settings.oauth_enabled}
          disabled={isLoading}
          onChange={() => handleChange('oauth_enabled')}
        />
      </Card>
    </section>
  );
};

interface EnterpriceLabelProps {
  children?: ReactNode;
}

const EnterPriceLabel = ({ children }: EnterpriceLabelProps) => {
  return (
    <>
      <p>{children}</p>
      <Badge text="Enterprice" />
    </>
  );
};
