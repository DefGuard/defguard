import { useQuery } from '@tanstack/react-query';
import { Link, useNavigate } from '@tanstack/react-router';
import { businessBadgeProps } from '../../../../shared/components/badges/BusinessBadge';
import { SettingsLayout } from '../../../../shared/components/SettingsLayout/SettingsLayout';
import { SectionSelect } from '../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { TooltipContent } from '../../../../shared/defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../../shared/defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../../shared/defguard-ui/providers/tooltip/TooltipTrigger';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { getLicenseInfoQueryOptions } from '../../../../shared/query';

export const SettingsGeneralTab = () => {
  const navigate = useNavigate();

  const { data: licenseInfo } = useQuery(getLicenseInfoQueryOptions);

  return (
    <SettingsLayout id="general-settings">
      <Link to="/settings/instance">
        <SectionSelect
          image="customization"
          title="Instance settings"
          content="Configure your instance name and branding settings. Add a logo to personalize the interface and make it easily recognizable to your users."
        />
      </Link>
      <SizedBox height={ThemeSpacing.Xl} />
      <TooltipProvider>
        <TooltipTrigger>
          <SectionSelect
            image="proxy-management"
            title="Proxy management"
            content="Configure your proxy settings and manage all proxy endpoints. Adjust connection rules and ensure your traffic is routed exactly as required."
            disabled
          />
        </TooltipTrigger>
        <TooltipContent>
          <p>{`Not implemented`}</p>
        </TooltipContent>
      </TooltipProvider>
      <SizedBox height={ThemeSpacing.Xl} />
      <SectionSelect
        image="behavior"
        title="Client behavior"
        content="Manage how users interact with the Defguard client. Control device management permissions, configuration access, and traffic routing options."
        badgeProps={licenseInfo === null ? businessBadgeProps : undefined}
        onClick={() => {
          navigate({ to: '/settings/client' });
        }}
      />
    </SettingsLayout>
  );
};
