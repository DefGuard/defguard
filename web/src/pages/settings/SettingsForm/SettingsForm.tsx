import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef } from 'react';
import { useForm } from 'react-hook-form';
import { SubmitHandler } from 'react-hook-form';
import shallow from 'zustand/shallow';

import { FormCheckBox } from '../../../shared/components/Form/FormCheckBox/FormCheckBox';
import { FormInput } from '../../../shared/components/Form/FormInput/FormInput';
import { useAppStore } from '../../../shared/hooks/store/useAppStore';
import { useSettingsFormStore } from '../../../shared/hooks/store/useSettingsFormStore';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../shared/mutations';
import { QueryKeys } from '../../../shared/queries';
import { Settings } from '../../../shared/types';

export const SettingsForm = () => {
  const toaster = useToaster();
  const {
    settings: { editSettings },
  } = useApi();

  const submitButton = useRef<HTMLButtonElement | null>(null);
  const [settings, setAppStore] = useAppStore(
    (state) => [state.settings, state.setAppStore],
    shallow
  );
  const queryClient = useQueryClient();
  const { mutate, isLoading } = useMutation(
    [MutationKeys.EDIT_SETTINGS],
    editSettings,
    {
      onSuccess: (_, request) => {
        queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
        toaster.success('Settings edited');
        setSettingsFormState({ editMode: false });
        setAppStore({ settings: request });
      },
      onError: () => {
        toaster.error('Error occured!', 'Please contact administrator');
      },
    }
  );
  const { control, handleSubmit } = useForm<Settings>({
    //resolver: yupResolver(schema),
    mode: 'all',
    defaultValues: {
      web3_enabled: settings?.web3_enabled,
      oauth_enabled: settings?.oauth_enabled,
      openid_enabled: settings?.openid_enabled,
      ldap_enabled: settings?.ldap_enabled,
      worker_enabled: settings?.worker_enabled,
      wireguard_enabled: settings?.wireguard_enabled,
      webhooks_enabled: settings?.webhooks_enabled,
      challenge_template: settings?.challenge_template,
    },
  });
  const [editMode, submitSubject, setSettingsFormState] = useSettingsFormStore(
    (state) => [state.editMode, state.submitSubject, state.setState]
  );

  const onSubmit: SubmitHandler<Settings> = (data) => mutate(data);

  useEffect(() => {
    if (submitButton && submitButton.current) {
      const sub = submitSubject.subscribe(() => {
        submitButton.current?.click();
      });
      return () => sub.unsubscribe();
    }
  }, [submitSubject]);
  return (
    <form id="settings-form" onSubmit={handleSubmit(onSubmit)}>
      <div className="row">
        <FormCheckBox
          label="Blockchain"
          controller={{ control, name: 'web3_enabled' }}
          disabled={!editMode}
        />
        <div className="enterprise">
          <FormCheckBox
            label="Oauth2"
            controller={{ control, name: 'oauth_enabled' }}
            disabled={!editMode}
          />
          <span>Enterprise</span>
        </div>
        <div className="enterprise">
          <FormCheckBox
            label="OpenID connect"
            controller={{ control, name: 'openid_enabled' }}
            disabled={!editMode}
          />
          <span>Enterprise</span>
        </div>
        <div className="enterprise">
          <FormCheckBox
            label="OpenLDAP"
            controller={{ control, name: 'ldap_enabled' }}
            disabled={!editMode}
          />
          <span>Enterprise</span>
        </div>
        <div className="enterprise">
          <FormCheckBox
            label="YubiBridge"
            controller={{ control, name: 'worker_enabled' }}
            disabled={!editMode}
          />
          <span>Enterprise</span>
        </div>
        <FormCheckBox
          label="Webhooks"
          controller={{ control, name: 'webhooks_enabled' }}
          disabled={!editMode}
        />
        <FormCheckBox
          label="Wireguard VPN"
          controller={{ control, name: 'wireguard_enabled' }}
          disabled={!editMode}
        />
        <FormInput
          outerLabel="Challenge template"
          controller={{ control, name: 'challenge_template' }}
          disabled={isLoading || !editMode}
          required
        />
      </div>
      <button type="submit" className="hidden" ref={submitButton} />
    </form>
  );
};
