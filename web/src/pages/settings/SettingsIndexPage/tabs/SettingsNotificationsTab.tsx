import { Link } from '@tanstack/react-router';
import { SettingsLayout } from '../../../../shared/components/SettingsLayout/SettingsLayout';
import { SectionSelect } from '../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { useApp } from '../../../../shared/hooks/useApp';
import { configuredBadge, notConfiguredBadge } from '../types';

export const SettingsNotificationsTab = () => {
  const smtp = useApp((s) => s.appInfo.smtp_enabled);

  return (
    <SettingsLayout>
      <Link to="/settings/smtp">
        <SectionSelect
          image="smtp"
          title="SMTP server configuration"
          content="Configure your SMTP server to enable email notifications and system alerts. Enter the connection details to ensure reliable message delivery."
          badgeProps={smtp ? configuredBadge : notConfiguredBadge}
        />
      </Link>
      <SizedBox height={ThemeSpacing.Xl} />
      <Link to="/settings/gateway-notifications">
        <SectionSelect
          image="gateway-notifications"
          title="Gateway notifications"
          content="Manage how and when your gateway sends notifications. Configure alert types, delivery methods, and recipients to stay informed about important events. "
        />
      </Link>
    </SettingsLayout>
  );
};
