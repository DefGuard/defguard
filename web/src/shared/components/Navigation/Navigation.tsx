import { useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import { CounterLabel } from '../../defguard-ui/components/CounterLabel/CounterLabel';
import { Icon, IconKind } from '../../defguard-ui/components/Icon';
import type { IconKindValue } from '../../defguard-ui/components/Icon/icon-types';
import { IconButton } from '../../defguard-ui/components/IconButton/IconButton';
import { useApp } from '../../hooks/useApp';
import { useAuth } from '../../hooks/useAuth';
import { NavLogo } from './assets/NavLogo';
import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { Link, type LinkProps } from '@tanstack/react-router';
import { type LicenseInfo, LicenseTier, type LicenseTierValue } from '../../api/types';
import { Fold } from '../../defguard-ui/components/Fold/Fold';
import { TooltipContent } from '../../defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../defguard-ui/providers/tooltip/TooltipTrigger';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import {
  getAliasesCountQueryOptions,
  getDestinationsCountQueryOptions,
  getLicenseInfoQueryOptions,
  getRulesCountQueryOptions,
  videoTutorialsQueryOptions,
} from '../../query';
import { canUseBusinessFeature } from '../../utils/license';
import { NavTutorialsButton } from '../../video-tutorials/components/widget/NavTutorialsButton/NavTutorialsButton';

interface NavGroupProps {
  id: string;
  label: string;
  items: NavItemProps[];
  licenseInfo?: LicenseInfo | null;
}

interface NavItemProps {
  id: string;
  label: string;
  icon: IconKindValue;
  link: LinkProps['to'];
  licenseTier?: LicenseTierValue;
  license?: LicenseInfo | null;
  testId?: string;
  pendingCount?: number;
}

const navigationConfig: NavGroupProps[] = [
  {
    id: 'vpn',
    label: m.cmp_nav_group_vpn(),
    items: [
      {
        id: 'overview',
        icon: 'pie-chart',
        label: m.cmp_nav_item_overview(),
        link: '/vpn-overview',
      },
      {
        id: 'locations',
        icon: 'location-tracking',
        label: m.cmp_nav_item_locations(),
        link: '/locations',
      },
    ],
  },
  {
    id: 'identity',
    label: m.cmp_nav_group_identity(),
    items: [
      {
        id: 'users',
        icon: 'users',
        label: m.cmp_nav_item_users(),
        link: '/users',
      },
      {
        id: 'groups',
        icon: 'groups',
        label: m.cmp_nav_item_groups(),
        link: '/groups',
        testId: 'groups',
      },
      {
        id: 'enrollment',
        icon: 'key',
        label: m.cmp_nav_item_enrollment(),
        link: '/enrollment',
      },
    ],
  },
  {
    id: 'firewall',
    label: m.cmp_nav_group_firewall(),
    items: [
      {
        id: 'rules',
        icon: 'rules',
        label: m.cmp_nav_item_rules(),
        link: '/acl/rules',
        licenseTier: LicenseTier.Business,
      },
      {
        id: 'destinations',
        icon: 'gateway',
        label: m.cmp_nav_item_destinations(),
        link: '/acl/destinations',
        licenseTier: LicenseTier.Business,
      },
      {
        id: 'aliases',
        icon: 'access-settings',
        label: m.cmp_nav_item_aliases(),
        link: '/acl/aliases',
        licenseTier: LicenseTier.Business,
      },
    ],
  },
  {
    id: 'integrations',
    label: m.cmp_nav_group_integrations(),
    items: [
      {
        id: 'activity_log',
        icon: 'activity',
        label: m.cmp_nav_item_activity_log(),
        link: '/activity',
      },
      {
        id: 'network_devices',
        icon: 'devices',
        label: m.cmp_nav_item_network_devices(),
        link: '/network-devices',
      },
      {
        id: 'openid',
        icon: 'openid',
        label: m.cmp_nav_item_openid(),
        link: '/openid',
      },
      {
        id: 'webhooks',
        icon: 'webhooks',
        label: m.cmp_nav_item_webhooks(),
        link: '/webhooks',
      },
    ],
  },
  {
    id: 'admin',
    label: m.cmp_nav_group_admin(),
    items: [
      {
        id: 'settings',
        icon: 'settings',
        label: m.cmp_nav_item_settings(),
        link: '/settings',
      },
      {
        id: 'support',
        icon: 'support',
        label: m.cmp_nav_item_support(),
        link: '/support',
      },
      {
        id: 'edges',
        icon: 'globe',
        label: m.cmp_nav_item_edges(),
        link: '/edges',
      },
    ],
  },
];

export const Navigation = () => {
  const isAdmin = useAuth((s) => s.isAdmin);
  const isOpen = useApp((s) => s.navigationOpen);

  const { data: licenseInfo } = useQuery({
    ...getLicenseInfoQueryOptions,
    enabled: isAdmin,
  });

  const { data: rulesCount } = useQuery({
    ...getRulesCountQueryOptions,
    enabled: isAdmin,
  });

  const { data: destinationsCount } = useQuery({
    ...getDestinationsCountQueryOptions,
    enabled: isAdmin,
  });

  const { data: aliasesCount } = useQuery({
    ...getAliasesCountQueryOptions,
    enabled: isAdmin,
  });

  const { data: videoTutorialsData } = useQuery(videoTutorialsQueryOptions);

  const navigationGroups = useMemo(() => {
    const pendingCounts = {
      rules: rulesCount?.pending,
      destinations: destinationsCount?.pending,
      aliases: aliasesCount?.pending,
    };

    return navigationConfig.map((group) => ({
      ...group,
      items: group.items.map((item) => ({
        ...item,
        pendingCount: pendingCounts[item.id as keyof typeof pendingCounts],
      })),
    }));
  }, [aliasesCount, destinationsCount, rulesCount]);

  if (!isAdmin || !isOpen) return null;
  return (
    <div className="navigation">
      <div className="top">
        <NavLogo />
        <div className="control">
          <IconButton
            icon="hamburger"
            onClick={() => {
              useApp.setState({
                navigationOpen: false,
              });
            }}
          />
        </div>
      </div>
      <div className="groups">
        {navigationGroups.map((group) => (
          <NavGroup key={group.id} {...group} licenseInfo={licenseInfo} />
        ))}
      </div>
      <div className="bottom">
        {videoTutorialsData && (
          <div className="nav-group">
            <NavTutorialsButton />
          </div>
        )}
      </div>
    </div>
  );
};

const NavGroup = ({ items, label, licenseInfo }: NavGroupProps) => {
  const [isOpen, setIsOpen] = useState(true);
  return (
    <div className="nav-group">
      <div
        className="track"
        onClick={() => {
          setIsOpen((s) => !s);
        }}
      >
        <Icon icon="arrow-small" rotationDirection={isOpen ? 'down' : 'right'} />
        <p>{label}</p>
      </div>
      <Fold open={isOpen}>
        <div className="items">
          {items.map((item) => (
            <NavItem key={item.id} {...item} license={licenseInfo} />
          ))}
        </div>
      </Fold>
    </div>
  );
};

const NavItem = ({
  icon,
  link,
  label,
  testId,
  license,
  licenseTier,
  pendingCount,
}: NavItemProps) => {
  const showLock = useMemo(() => {
    if (licenseTier === undefined) {
      return isPresent(licenseTier);
    }

    if (licenseTier !== undefined && licenseTier === LicenseTier.Business) {
      return !canUseBusinessFeature(license as LicenseInfo | null).result;
    }

    if (licenseTier !== undefined && licenseTier === LicenseTier.Enterprise) {
      return !canUseBusinessFeature(license as LicenseInfo | null).result;
    }

    return false;
  }, [license, licenseTier]);

  const showPending = !showLock && isPresent(pendingCount) && pendingCount > 0;
  const showRight = showPending || (showLock && isPresent(licenseTier));

  return (
    <Link to={link} className="nav-item" data-testid={testId}>
      <Icon icon={icon} />
      <span>{label}</span>
      {showRight && (
        <div className="right">
          {showPending && <CounterLabel value={pendingCount} variant="warning" />}
          {showLock && isPresent(licenseTier) && (
            <TooltipProvider>
              <TooltipTrigger>
                <Icon icon={IconKind.LockClosed} size={16} />
              </TooltipTrigger>
              <TooltipContent>
                <p>{`This is ${licenseTier ?? 'Unknown tier'} feature`}</p>
              </TooltipContent>
            </TooltipProvider>
          )}
        </div>
      )}
    </Link>
  );
};
