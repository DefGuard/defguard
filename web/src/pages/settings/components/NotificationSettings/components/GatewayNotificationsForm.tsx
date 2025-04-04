import parse from 'html-react-parser';
import { Control } from 'react-hook-form';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormCheckBox } from '../../../../../shared/defguard-ui/components/Form/FormCheckBox/FormCheckBox';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Helper } from '../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import { FormFields } from './NotificationSettingsForm';

export const GatewayNotificationsForm = ({
  control,
  isLoading,
}: {
  control: Control<FormFields>;
  isLoading: boolean;
}) => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.gatewayNotifications;
  const smtpConfigured = useAppStore((s) => Boolean(s.appInfo?.smtp_enabled));

  return (
    <div>
      <h3 className="subsection-header">{localLL.sections.gateway()}</h3>
      <div className="checkbox-column">
        <div className="helper-row">
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
        <div className="helper-row">
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
    </div>
  );
};
