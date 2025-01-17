import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
import { useLocation } from 'react-router';
import { useBreakpoint } from 'use-breakpoint';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../i18n/i18n-react';
import SvgIconNavGroups from '../../shared/components/svg/IconNavGroups';
import SvgIconNavKey from '../../shared/components/svg/IconNavKey';
import SvgIconNavOpenId from '../../shared/components/svg/IconNavOpenid';
import SvgIconNavProfile from '../../shared/components/svg/IconNavProfile';
import SvgIconNavProvisioners from '../../shared/components/svg/IconNavProvisioners';
import SvgIconNavSettings from '../../shared/components/svg/IconNavSettings';
import SvgIconNavSupport from '../../shared/components/svg/IconNavSupport';
import SvgIconNavUsers from '../../shared/components/svg/IconNavUsers';
import SvgIconNavVpn from '../../shared/components/svg/IconNavVpn';
import SvgIconNavWebhooks from '../../shared/components/svg/IconNavWebhooks';
import { deviceBreakpoints } from '../../shared/constants';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import { useUserProfileStore } from '../../shared/hooks/store/useUserProfileStore';
import useApi from '../../shared/hooks/useApi';
import { QueryKeys } from '../../shared/queries';
import { User } from '../../shared/types';
import { invalidateMultipleQueries } from '../../shared/utils/invalidateMultipleQueries';
import { DevicePageNavigationIcon } from './components/DevicesPageNavigationIcon';
import { NavigationDesktop } from './components/NavigationDesktop/NavigationDesktop';
import { NavigationMobile } from './components/NavigationMobile/NavigationMobile';
import { navigationExcludedRoutes } from './config';
import { useNavigationStore } from './hooks/useNavigationStore';
import { NavigationItem, NavigationItems } from './types';

export const Navigation = () => {
  const { pathname } = useLocation();
  const { LL } = useI18nContext();
  const [currentUser, resetAuthStore] = useAuthStore(
    (state) => [state.user, state.resetState],
    shallow,
  );
  const setStore = useNavigationStore((state) => state.setState);
  const networksPresent = useAppStore((state) => state.appInfo?.network_present);
  const resetUserProfile = useUserProfileStore((state) => state.reset);
  const queryClient = useQueryClient();

  const {
    auth: { logout },
  } = useApi();

  const { mutate: logOutMutation } = useMutation({
    mutationFn: logout,
    onSuccess: () => {
      resetAuthStore();
      resetUserProfile();
      setStore({ isOpen: false });
    },
  });

  const settings = useAppStore((state) => state.settings);
  const { breakpoint } = useBreakpoint(deviceBreakpoints);

  const navItems = useMemo((): NavigationItems => {
    if (!currentUser) {
      return {
        middle: [],
        bottom: [],
      };
    }

    const overviewLink = networksPresent ? '/admin/overview' : '/admin/wizard';

    let bottom: NavigationItem[] = [
      {
        title: LL.navigation.bar.settings(),
        linkPath: '/admin/settings',
        icon: <SvgIconNavSettings />,
        adminOnly: true,
        enabled: true,
      },
      {
        title: LL.navigation.bar.support(),
        icon: <SvgIconNavSupport />,
        linkPath: '/support',
        adminOnly: false,
        enabled: true,
        className: 'support',
      },
    ];
    let middle: NavigationItem[] = [
      {
        title: LL.navigation.bar.overview(),
        linkPath: overviewLink,
        icon: <SvgIconNavVpn />,
        adminOnly: true,
        enabled: settings?.wireguard_enabled,
      },
      {
        title: LL.navigation.bar.users(),
        linkPath: '/admin/users',
        icon: <SvgIconNavUsers />,
        adminOnly: true,
        enabled: true,
      },
      {
        title: LL.navigation.bar.groups(),
        linkPath: '/admin/groups',
        icon: <SvgIconNavGroups />,
        adminOnly: true,
        enabled: true,
      },
      {
        title: LL.navigation.bar.devices(),
        linkPath: '/admin/devices',
        icon: <DevicePageNavigationIcon />,
        adminOnly: true,
        enabled: true,
      },
      {
        title: LL.navigation.bar.openId(),
        linkPath: '/admin/openid',
        icon: <SvgIconNavOpenId />,
        adminOnly: true,
        enabled: settings?.openid_enabled,
      },
      {
        title: LL.navigation.bar.webhooks(),
        linkPath: '/admin/webhooks',
        icon: <SvgIconNavWebhooks />,
        adminOnly: true,
        enabled: settings?.webhooks_enabled,
      },
      {
        title: LL.navigation.bar.provisioners(),
        linkPath: '/admin/provisioners',
        icon: <SvgIconNavProvisioners />,
        adminOnly: true,
        enabled: settings?.worker_enabled,
      },
      {
        title: LL.navigation.bar.enrollment(),
        linkPath: '/admin/enrollment',
        icon: <SvgIconNavKey />,
        adminOnly: true,
        enabled: true,
      },
      {
        title: LL.navigation.bar.myProfile(),
        linkPath: `/me`,
        icon: <SvgIconNavProfile />,
        adminOnly: false,
        enabled: true,
        onClick: () => {
          resetUserProfile();
          invalidateMultipleQueries(queryClient, [
            [QueryKeys.FETCH_ME],
            [QueryKeys.FETCH_USER_PROFILE],
          ]);
        },
      },
    ];
    middle = filterNavItems(middle, currentUser);
    bottom = filterNavItems(bottom, currentUser);
    return {
      middle,
      bottom,
    };
  }, [
    LL.navigation.bar,
    currentUser,
    networksPresent,
    queryClient,
    resetUserProfile,
    settings?.openid_enabled,
    settings?.webhooks_enabled,
    settings?.wireguard_enabled,
    settings?.worker_enabled,
  ]);

  const renderNav = useMemo(() => {
    for (const path of navigationExcludedRoutes) {
      if (pathname.includes(path)) {
        return false;
      }
    }
    return true;
  }, [pathname]);

  if (!renderNav) return null;

  return (
    <>
      {breakpoint === 'desktop' && (
        <NavigationDesktop navItems={navItems} onLogout={() => logOutMutation()} />
      )}
      {breakpoint !== 'desktop' && (
        <NavigationMobile navItems={navItems} onLogout={() => logOutMutation()} />
      )}
    </>
  );
};

const filterNavItems = (items: NavigationItem[], currentUser: User): NavigationItem[] =>
  items
    .filter((item) => item.enabled)
    .filter((item) => {
      if (item.adminOnly) {
        return currentUser ? currentUser.is_admin : false;
      }
    });
