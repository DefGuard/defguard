import './styles.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import parse from 'html-react-parser';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../shared/components/svg/IconCheckmarkWhite';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { LabeledCheckbox } from '../../../../../shared/defguard-ui/components/Layout/LabeledCheckbox/LabeledCheckbox';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { useSettingsPage } from '../../../hooks/useSettingsPage';

type FormFields = {
  gateway_disconnect_notifications_enabled: boolean;
  gateway_disconnect_notifications_inactivity_threshold: number;
  gateway_disconnect_notifications_reconnect_notification_enabled: boolean;
};

export const GatewayNotificationsForm = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.gatewayNotifications;
  const settings = useSettingsPage((state) => state.settings);

  const toaster = useToaster();

  const {
    settings: { patchSettings },
  } = useApi();

  const queryClient = useQueryClient();

  const { mutate, isLoading } = useMutation(patchSettings, {
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
      queryClient.invalidateQueries([QueryKeys.FETCH_APP_INFO]);
      toaster.success(LL.settingsPage.messages.editSuccess());
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
      console.error(err);
    },
  });

  const zodSchema = useMemo(
    () =>
      z.object({
        gateway_disconnect_notifications_enabled: z.boolean(),
        gateway_disconnect_notifications_inactivity_threshold: z
          .number()
          .min(0, LL.form.error.minimumValue({ value: 0 })),
        gateway_disconnect_notifications_reconnect_notification_enabled: z.boolean(),
      }),
    [LL.form],
  );

  const defaultValues = useMemo(() => {
    const res: FormFields = {
      gateway_disconnect_notifications_enabled:
        settings?.gateway_disconnect_notifications_enabled ?? false,
      gateway_disconnect_notifications_inactivity_threshold:
        settings?.gateway_disconnect_notifications_inactivity_threshold ?? 5,
      gateway_disconnect_notifications_reconnect_notification_enabled:
        settings?.gateway_disconnect_notifications_reconnect_notification_enabled ??
        false,
    };
    return res;
  }, [settings]);

  const { control, handleSubmit } = useForm<FormFields>({
    defaultValues,
    mode: 'all',
    resolver: zodResolver(zodSchema),
  });

  const onSubmit: SubmitHandler<FormFields> = (data) => {
    mutate(data);
  };

  if (!settings) return null;

  return (
    <section id="gateway-notifications-settings">
      <header>
        <h2>{localLL.header()}</h2>
        <Helper>{parse(localLL.helper())}</Helper>
        <div className="controls">
          <Button
            form="gateway-notifications-form"
            text={localLL.form.submit()}
            icon={<IconCheckmarkWhite />}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.SAVE}
            loading={isLoading}
            type="submit"
          />
        </div>
      </header>
      <form id="gateway-notifications-form" onSubmit={handleSubmit(onSubmit)}>
        <div className="checkbox-row">
          <LabeledCheckbox
            disabled={isLoading}
            label={localLL.form.fields.disconnectNotificationsEnabled.label()}
            value={settings.gateway_disconnect_notifications_enabled}
            onChange={() =>
              mutate({
                gateway_disconnect_notifications_enabled:
                  !settings.gateway_disconnect_notifications_enabled,
              })
            }
          />
          <Helper>
            {parse(LL.settingsPage.enterprise.fields.deviceManagement.helper())}
          </Helper>
        </div>
        <FormInput
          type="number"
          controller={{
            control,
            name: 'gateway_disconnect_notifications_inactivity_threshold',
          }}
          label={localLL.form.fields.inactivityThreshold.label()}
          labelExtras={
            <Helper>{parse(localLL.form.fields.inactivityThreshold.help())}</Helper>
          }
          disabled={isLoading || !settings.gateway_disconnect_notifications_enabled}
          required
        />
        <div className="checkbox-row">
          <LabeledCheckbox
            disabled={isLoading || !settings.gateway_disconnect_notifications_enabled}
            label={localLL.form.fields.reconnectNotificationsEnabled.label()}
            value={
              settings.gateway_disconnect_notifications_reconnect_notification_enabled
            }
            onChange={() =>
              mutate({
                gateway_disconnect_notifications_reconnect_notification_enabled:
                  !settings.gateway_disconnect_notifications_reconnect_notification_enabled,
              })
            }
          />
          <Helper>
            {parse(localLL.form.fields.reconnectNotificationsEnabled.help())}
          </Helper>
        </div>
      </form>
    </section>
  );
};
