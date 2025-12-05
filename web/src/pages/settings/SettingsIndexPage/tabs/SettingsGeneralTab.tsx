import { SettingsLayout } from '../../../../shared/components/SettingsLayout/SettingsLayout';
import { SectionSelect } from '../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';

export const SettingsGeneralTab = () => {
  return (
    <SettingsLayout id="general-settings">
      <SectionSelect
        image="customization"
        title="Instance settings"
        content="Configure your instance name and branding settings. Add a logo to personalize the interface and make it easily recognizable to your users."
      />
      <SizedBox height={ThemeSpacing.Xl} />
      <SectionSelect
        image="proxy-management"
        title="Proxy management"
        content="Configure your proxy settings and manage all proxy endpoints. Adjust connection rules and ensure your traffic is routed exactly as required."
      />
      <SizedBox height={ThemeSpacing.Xl} />
      <SectionSelect
        image="behavior"
        title="Client behavior"
        content="Manage how users interact with the Defguard client. Control device management permissions, configuration access, and traffic routing options."
      />
    </SettingsLayout>
  );
};
