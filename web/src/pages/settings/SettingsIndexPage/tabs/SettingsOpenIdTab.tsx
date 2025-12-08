import { Link } from '@tanstack/react-router';
import { SettingsLayout } from '../../../../shared/components/SettingsLayout/SettingsLayout';
import { SectionSelect } from '../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSize } from '../../../../shared/defguard-ui/types';

export const SettingsOpenIdTab = () => {
  return (
    <SettingsLayout>
      <Link to="/settings/openid/general">
        <SectionSelect
          image="integrations"
          title="General settings"
          content="Configure your instance name and branding settings. Add a logo to personalize the interface and make it easily recognizable to your users."
        />
      </Link>
      <SizedBox height={ThemeSize.Xl} />
      <SectionSelect
        image="id-providers"
        title="External OpenID settings"
        content="Manage how users interact with the Defguard client. Control device management permissions, configuration access, and traffic routing options."
      />
    </SettingsLayout>
  );
};
