import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'react-toastify';
import shallow from 'zustand/shallow';

import { CheckBox } from '../../../shared/components/layout/Checkbox/CheckBox';
import ToastContent, {
  ToastType,
} from '../../../shared/components/Toasts/ToastContent';
import { useAppStore } from '../../../shared/hooks/store/useAppStore';
import useApi from '../../../shared/hooks/useApi';
import { MutationKeys } from '../../../shared/mutations';
import { QueryKeys } from '../../../shared/queries';

export const SettingsForm = () => {
  const {
    settings: { editSettings },
  } = useApi();

  const [settings, setAppStore] = useAppStore(
    (state) => [state.settings, state.setAppStore],
    shallow
  );
  const queryClient = useQueryClient();
  const { mutate } = useMutation([MutationKeys.EDIT_SETTINGS], editSettings, {
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
      toast(
        <ToastContent type={ToastType.SUCCESS} message={'Settings edited'} />
      );
    },
    onError: () => {
      toast(
        <ToastContent
          type={ToastType.ERROR}
          message={'Unexpected error occured'}
        />
      );
    },
  });

  const handleChange = (e: boolean, feature: string) => {
    if (settings !== undefined) {
      switch (feature) {
        case 'vpn':
          settings.wireguard_enabled = e;
          break;
        case 'ldap':
          settings.ldap_enabled = e;
          break;
        case 'openid':
          settings.openid_enabled = e;
          break;
        case 'webhooks':
          settings.webhooks_enabled = e;
          break;
        case 'web3':
          settings.web3_enabled = e;
          break;
        case 'oauth':
          settings.oauth_enabled = e;
          break;
        case 'worker':
          settings.worker_enabled = e;
          break;
      }
      setAppStore({ settings });
      mutate(settings);
    }
  };

  return (
    <div className="row">
      <CheckBox
        label="Blockchain"
        value={Number(settings?.web3_enabled)}
        onChange={(e) => handleChange(Boolean(e), 'web3')}
      />
      <CheckBox
        label="Oauth2"
        value={Number(settings?.oauth_enabled)}
        onChange={(e) => handleChange(Boolean(e), 'oauth')}
      />
      <CheckBox
        label="OpenID Connect"
        value={Number(settings?.openid_enabled)}
        onChange={(e) => handleChange(Boolean(e), 'openid')}
      />
      <CheckBox
        label="OpenLDAP"
        value={Number(settings?.ldap_enabled)}
        onChange={(e) => handleChange(Boolean(e), 'ldap')}
      />
      <CheckBox
        label="YubiBridge"
        value={Number(settings?.worker_enabled)}
        onChange={(e) => handleChange(Boolean(e), 'worker')}
      />
      <CheckBox
        label="Webhooks"
        value={Number(settings?.webhooks_enabled)}
        onChange={(e) => handleChange(Boolean(e), 'webhooks')}
      />
      <CheckBox
        label="Wireguard VPN"
        value={Number(settings?.wireguard_enabled)}
        onChange={(e) => handleChange(Boolean(e), 'vpn')}
      />
    </div>
  );
};
