import './styles.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import parse from 'html-react-parser';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../shared/components/svg/IconCheckmarkWhite';
import { FormCheckBox } from '../../../../../shared/defguard-ui/components/Form/FormCheckBox/FormCheckBox';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { ApiError } from '../../../../../shared/types';
import { invalidateMultipleQueries } from '../../../../../shared/utils/invalidateMultipleQueries';
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

  const smtpConfigured = useAppStore((s) => Boolean(s.appInfo?.smtp_enabled));

  const queryClient = useQueryClient();

  const { mutate, isPending: isLoading } = useMutation({
    mutationFn: patchSettings,
    onSuccess: () => {
      invalidateMultipleQueries(queryClient, [
        [QueryKeys.FETCH_APP_INFO],
        [QueryKeys.FETCH_SETTINGS],
      ]);
      toaster.success(LL.settingsPage.messages.editSuccess());
    },
    onError: (err: ApiError) => {
      toaster.error(err.response?.data.msg || LL.messages.error());
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
            disabled={!smtpConfigured}
            type="submit"
          />
        </div>
      </header>
      <form id="gateway-notifications-form" onSubmit={handleSubmit(onSubmit)}>
        <div className="checkbox-row">
          <FormCheckBox
            disabled={isLoading || !smtpConfigured}
            label={localLL.form.fields.disconnectNotificationsEnabled.label()}
            controller={{
              control,
              name: 'gateway_disconnect_notifications_enabled',
            }}
            labelPlacement="right"
          />
          <Helper>
            {parse(localLL.form.fields.disconnectNotificationsEnabled.help())}
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
          disabled={isLoading || !smtpConfigured}
          required
        />
        <div className="checkbox-row">
          <FormCheckBox
            disabled={isLoading || !smtpConfigured}
            label={localLL.form.fields.reconnectNotificationsEnabled.label()}
            controller={{
              control,
              name: 'gateway_disconnect_notifications_reconnect_notification_enabled',
            }}
            labelPlacement="right"
          />
          <Helper>
            {parse(localLL.form.fields.reconnectNotificationsEnabled.help())}
          </Helper>
        </div>
      </form>
    </section>
  );
};
