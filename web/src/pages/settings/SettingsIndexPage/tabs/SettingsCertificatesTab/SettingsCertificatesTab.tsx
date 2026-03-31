import { Link } from '@tanstack/react-router';
import { m } from '../../../../../paraglide/messages';
import { SettingsLayout } from '../../../../../shared/components/SettingsLayout/SettingsLayout';
import { SectionSelect } from '../../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../shared/defguard-ui/types';

export const SettingsCertificatesTab = () => {
  return (
    <SettingsLayout id="certificates-settings">
      <Link to="/settings/ca">
        <SectionSelect
          image="certificate-authority"
          title={m.settings_certs_ca_title()}
          content={m.settings_certs_ca_description()}
        />
      </Link>
      <SizedBox height={ThemeSpacing.Xl} />
      <Link to="/settings/certs">
        <SectionSelect
          image="certificates"
          title={m.settings_certs_certs_title()}
          content={m.settings_certs_certs_description()}
        />
      </Link>
    </SettingsLayout>
  );
};
