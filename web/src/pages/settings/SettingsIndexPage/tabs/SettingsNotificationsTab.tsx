import { Link } from '@tanstack/react-router';
import { SettingsLayout } from '../../../../shared/components/SettingsLayout/SettingsLayout';
import { SectionSelect } from '../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';

export const SettingsNotificationsTab = () => {
  return (
    <SettingsLayout>
      <Link to="/settings/smtp">
        <SectionSelect
          image="smtp"
          title="SMTP server configuration"
          content="Configure your SMTP server to enable email notifications and system alerts. Enter the connection details to ensure reliable message delivery."
        />
      </Link>
      <SizedBox height={ThemeSpacing.Xl} />
      <SectionSelect
        image="gateway-notifications"
        title="Gateway notifications"
        content="Manage how and when your gateway sends notifications. Configure alert types, delivery methods, and recipients to stay informed about important events. "
      />
    </SettingsLayout>
  );
};
