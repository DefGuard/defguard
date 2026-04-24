import { useQuery } from '@tanstack/react-query';
import { Link, useNavigate } from '@tanstack/react-router';
import { m } from '../../../../paraglide/messages';
import { businessBadgeProps } from '../../../../shared/components/badges/BusinessBadge';
import {
  ContextualHelpKey,
  ContextualHelpSidebar,
} from '../../../../shared/components/ContextualHelp';
import { SettingsLayout } from '../../../../shared/components/SettingsLayout/SettingsLayout';
import { SectionSelect } from '../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { getLicenseInfoQueryOptions } from '../../../../shared/query';

export const SettingsGeneralTab = () => {
  const navigate = useNavigate();

  const { data: licenseInfo } = useQuery(getLicenseInfoQueryOptions);

  return (
    <SettingsLayout
      id="general-settings"
      suggestion={<ContextualHelpSidebar pageKey={ContextualHelpKey.SettingsGeneral} />}
    >
      <Link to="/settings/instance">
        <SectionSelect
          image="customization"
          title={m.settings_instance_title()}
          content={m.settings_general_section_instance_content()}
        />
      </Link>
      <SizedBox height={ThemeSpacing.Xl} />
      <SectionSelect
        image="behavior"
        title={m.settings_breadcrumb_client_behavior()}
        content={m.settings_general_section_client_behavior_content()}
        badgeProps={licenseInfo === null ? businessBadgeProps : undefined}
        onClick={() => {
          navigate({ to: '/settings/client' });
        }}
      />
    </SettingsLayout>
  );
};
