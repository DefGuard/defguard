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

export const ModulesCard = () => {
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
        toaster.success('Settings changed.');
      },
      onError: () => {
        toaster.error('Error occured!', 'Please contact administrator');
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
        <h2>Modules Visibility</h2>
        <Helper>
          <p>
            If your not using some modules you can disable their visibility.
          </p>{' '}
          <a href="defguard.gitbook.io" target="_blank">
            Read more in documentation.
          </a>
        </Helper>
      </header>
      <Card>
        <CheckBox
          label="WireGuard VPN"
          value={settings.wireguard_enabled}
          disabled={isLoading}
          onChange={() => handleChange('wireguard_enabled')}
        />
        <CheckBox
          label="Webhooks"
          value={settings.webhooks_enabled}
          disabled={isLoading}
          onChange={() => handleChange('webhooks_enabled')}
        />
        <CheckBox
          label="Web3"
          value={settings.web3_enabled}
          disabled={isLoading}
          onChange={() => handleChange('web3_enabled')}
        />
        <CheckBox
          label={<EnterPriceLabel>YubiBridge</EnterPriceLabel>}
          value={settings.worker_enabled}
          disabled={isLoading}
          onChange={() => handleChange('worker_enabled')}
        />
        <CheckBox
          disabled={isLoading}
          label={<EnterPriceLabel>OpenID connect</EnterPriceLabel>}
          value={settings.openid_enabled}
          onChange={() => handleChange('openid_enabled')}
        />
        <CheckBox
          label={<EnterPriceLabel>OAuth2</EnterPriceLabel>}
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
