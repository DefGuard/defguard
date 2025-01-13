import parse from 'html-react-parser';
import { BigInfoBox } from '../../../../shared/defguard-ui/components/Layout/BigInfoBox/BigInfoBox';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import { GatewayNotificationsForm } from './components/GatewayNotificationsForm';
import { useI18nContext } from '../../../../i18n/i18n-react';

export const GatewayNotificationsSettings = () => {
  const { LL } = useI18nContext();
  const appInfo = useAppStore((s) => s.appInfo);

  if (!appInfo) return null;

  return (
    <>
      {!appInfo.smtp_enabled && (
        <div className="license-not-required-container">
          <BigInfoBox
            message={parse(LL.settingsPage.gatewayNotifications.smtpWarning())}
          />
        </div>
      )}

      <div className="left">
        <GatewayNotificationsForm />
      </div>
      <div className="right"></div>
    </>
  );
};
