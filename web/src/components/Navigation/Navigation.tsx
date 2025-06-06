import './style.scss';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
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
import { NavigationActivityLogPageIcon } from './components/icons/NavigationActivityLogPageIcon';
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
  const isAdmin = useAuthStore((s) => s.user?.is_admin ?? false);

  const {
    auth: { logout },
    network: { getNetworks },
  } = useApi();

  const { data: networks } = useQuery({
    queryKey: ['network'],
    queryFn: getNetworks,
    enabled: isAdmin,
  });

  const onlyOneNetworkPresent = useMemo(() => {
    if (networks) {
      return networks.length === 1;
    }
    return false;
  }, [networks]);

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

    let overviewLink = '/admin/overview';

    if (!networksPresent) {
      overviewLink = '/admin/overview';
    }

    if (networks && onlyOneNetworkPresent) {
      const networkId = networks[0].id;
      overviewLink = `/admin/overview/${networkId}`;
    }

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
        title: LL.navigation.bar.acl(),
        linkPath: '/admin/acl',
        icon: (
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="25"
            viewBox="0 0 24 25"
            fill="none"
          >
            <path
              fillRule="evenodd"
              clipRule="evenodd"
              d="M9 0.5C6.23858 0.5 4 2.73872 4 5.50031V9.30324C2.2066 10.3407 1 12.2799 1 14.5009V21.5013C1 23.1583 2.34315 24.5015 4 24.5015H20C21.6569 24.5015 23 23.1583 23 21.5013V14.5009C23 11.187 20.3137 8.5005 17 8.5005H7C6.65929 8.5005 6.32521 8.5289 6 8.58346V5.50031C6 3.84335 7.34315 2.50012 9 2.50012H15C16.6569 2.50012 18 3.84335 18 5.50031H20C20 2.73872 17.7614 0.5 15 0.5H9ZM7 10.5006H17C19.2091 10.5006 21 12.2916 21 14.5009V21.5013C21 22.0536 20.5523 22.5014 20 22.5014H4C3.44772 22.5014 3 22.0536 3 21.5013V14.5009C3 12.2916 4.79086 10.5006 7 10.5006ZM13 17.2335C13.5978 16.8876 14 16.2413 14 15.5009C14 14.3963 13.1046 13.5008 12 13.5008C10.8954 13.5008 10 14.3963 10 15.5009C10 16.2413 10.4022 16.8876 11 17.2335V19.5C11 20.0523 11.4477 20.5 12 20.5C12.5523 20.5 13 20.0523 13 19.5V17.2335Z"
              fill="#0C8CE0"
            />
          </svg>
        ),
        adminOnly: true,
        enabled: true,
        enterpriseOnly: true,
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
        title: LL.navigation.bar.activity(),
        linkPath: '/admin/activity',
        icon: <NavigationActivityLogPageIcon />,
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
    networks,
    networksPresent,
    onlyOneNetworkPresent,
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
      } else {
        return true;
      }
    });
