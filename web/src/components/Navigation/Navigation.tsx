import './style.scss';

import { useMutation } from '@tanstack/react-query';
import { motion } from 'framer-motion';
import React, { useMemo, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { useBreakpoint } from 'use-breakpoint';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../i18n/i18n-react';
import { Divider } from '../../shared/components/layout/Divider/Divider';
import IconButton from '../../shared/components/layout/IconButton/IconButton';
import SvgDefguadNavLogo from '../../shared/components/svg/DefguadNavLogo';
import SvgDefguadNavLogoCollapsed from '../../shared/components/svg/DefguadNavLogoCollapsed';
import SvgIconArrowDoubleGrayLeft from '../../shared/components/svg/IconArrowDoubleGrayLeft';
import SvgIconEdit from '../../shared/components/svg/IconEditAlt';
import SvgIconHamburgerMenu from '../../shared/components/svg/IconHamburgerMenu';
import SvgIconNavLogout from '../../shared/components/svg/IconNavLogout';
import SvgIconNavOpenId from '../../shared/components/svg/IconNavOpenid';
import SvgIconNavProfile from '../../shared/components/svg/IconNavProfile';
import SvgIconNavProvisioners from '../../shared/components/svg/IconNavProvisioners';
import SvgIconNavUsers from '../../shared/components/svg/IconNavUsers';
import SvgIconNavVpn from '../../shared/components/svg/IconNavVpn';
import SvgIconNavWebhooks from '../../shared/components/svg/IconNavWebhooks';
import { deviceBreakpoints } from '../../shared/constants';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import { useNavigationStore } from '../../shared/hooks/store/useNavigationStore';
import useApi from '../../shared/hooks/useApi';
import { ApplicationVersion } from './ApplicationVersion/ApplicationVersion';
import { MobileNavModal } from './MobileNavModal/MobileNavModal';
import { NavigationLink } from './NavigationLink';

export interface NavigationItem {
  title: string;
  linkPath: string;
  icon?: React.ReactNode;
  allowedToView?: string[];
  enabled: boolean | undefined;
}

export const Navigation = () => {
  const { LL, locale } = useI18nContext();
  const [currentUser, resetAuthStore] = useAuthStore(
    (state) => [state.user, state.resetState],
    shallow
  );
  const [isMobileNavOpen, setMobileNavOpen] = useState(false);
  const [isNavigationOpen, setNavigationOpen] = useNavigationStore(
    (state) => [state.isNavigationOpen, state.setNavigationOpen, state.user],
    shallow
  );
  const [enableWizard] = useNavigationStore((state) => [state.enableWizard], shallow);
  const {
    auth: { logout },
  } = useApi();
  const logOutMutation = useMutation(logout, {
    onSuccess: () => {
      resetAuthStore();
    },
  });

  const settings = useAppStore((state) => state.settings);
  const { breakpoint } = useBreakpoint(deviceBreakpoints);

  const { pathname } = useLocation();

  const getPageTitle = useMemo(() => {
    if (pathname === '/admin/settings') {
      return LL.navigation.mobileTitles.settings();
    }
    if (pathname === '/admin/users' || pathname === '/admin/users/') {
      return LL.navigation.mobileTitles.users();
    }
    if (pathname.includes('/admin/users/') || pathname.includes('/me')) {
      return LL.navigation.mobileTitles.user();
    }
    if (pathname.includes('/admin/provisioners')) {
      return LL.navigation.mobileTitles.provisioners();
    }
    if (pathname.includes('/admin/webhooks')) {
      return LL.navigation.mobileTitles.webhooks();
    }
    if (pathname.includes('/admin/openid')) {
      return LL.navigation.mobileTitles.openId();
    }
    if (pathname.includes('/admin/overview')) {
      return LL.navigation.mobileTitles.overview();
    }
    if (pathname.includes('/admin/network')) {
      return LL.navigation.mobileTitles.networkSettings();
    }
    return '';
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pathname, locale]);

  const navItems: NavigationItem[] = useMemo(() => {
    let base: NavigationItem[] = [
      {
        title: LL.navigation.bar.overview(),
        linkPath: '/admin/overview',
        icon: <SvgIconNavVpn />,
        allowedToView: ['admin'],
        enabled: settings?.wireguard_enabled && !enableWizard,
      },
      {
        title: LL.navigation.bar.wizard(),
        linkPath: '/admin/wizard',
        icon: <SvgIconNavVpn />,
        allowedToView: ['admin'],
        enabled: settings?.wireguard_enabled && enableWizard,
      },
      {
        title: LL.navigation.bar.users(),
        linkPath: '/admin/users',
        icon: <SvgIconNavUsers />,
        allowedToView: ['admin'],
        enabled: true,
      },
      {
        title: LL.navigation.bar.openId(),
        linkPath: '/admin/openid',
        icon: <SvgIconNavOpenId />,
        allowedToView: ['admin'],
        enabled: settings?.openid_enabled,
      },
      {
        title: LL.navigation.bar.webhooks(),
        linkPath: '/admin/webhooks',
        icon: <SvgIconNavWebhooks />,
        allowedToView: ['admin'],
        enabled: settings?.webhooks_enabled,
      },
      {
        title: LL.navigation.bar.provisioners(),
        linkPath: '/admin/provisioners',
        icon: <SvgIconNavProvisioners />,
        allowedToView: ['admin'],
        enabled: settings?.worker_enabled,
      },
      {
        title: LL.navigation.bar.myProfile(),
        linkPath: `/me`,
        icon: <SvgIconNavProfile />,
        allowedToView: [],
        enabled: true,
      },
    ];
    base = base.filter((item) => {
      if (item.allowedToView && item.allowedToView.length) {
        if (currentUser) {
          for (const group of currentUser.groups) {
            if (item.allowedToView?.includes(group)) {
              return true;
            }
          }
          return false;
        } else {
          return false;
        }
      } else {
        return true;
      }
    });
    base = base.filter((item) => item.enabled);

    return base;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentUser, settings, locale]);

  return (
    <>
      {breakpoint !== 'desktop' ? (
        <nav className="nav-mobile">
          <SvgDefguadNavLogoCollapsed />
          <p className="page-title">{getPageTitle}</p>
          <IconButton className="hamburger-button" onClick={() => setMobileNavOpen(true)}>
            <SvgIconHamburgerMenu />
          </IconButton>
        </nav>
      ) : null}
      {breakpoint === 'desktop' ? (
        <>
          <button
            onClick={() => setNavigationOpen(!isNavigationOpen)}
            className={'nav-control-button' + (isNavigationOpen ? '' : ' collapsed')}
          >
            <SvgIconArrowDoubleGrayLeft />
          </button>
          <motion.nav
            className={'nav-container ' + (isNavigationOpen ? 'visible' : '')}
            layout
          >
            <section className="logo-container">
              {settings ? <img src={settings?.nav_logo_url} alt="logo" /> : null}
              <SvgDefguadNavLogo
                style={{ display: settings?.nav_logo_url ? 'none' : 'block' }}
              />
              <SvgDefguadNavLogoCollapsed />
            </section>
            <span className="divider"></span>
            <section className="links">
              {navItems.map((item) => (
                <NavigationLink key={item.linkPath} item={item} />
              ))}
            </section>
            <section className="links">
              <NavigationLink
                key={'/admin/settings'}
                item={{
                  title: LL.navigation.bar.settings(),
                  linkPath: '/admin/settings',
                  icon: <SvgIconEdit />,
                  allowedToView: ['admin'],
                  enabled: true,
                }}
              />
              <button
                data-testid="logout"
                className="log-out"
                onClick={() => logOutMutation.mutate()}
              >
                <SvgIconNavLogout />
                <span>{LL.navigation.bar.logOut()}</span>
              </button>
              {isNavigationOpen ? <Divider key="app-version-divider" /> : null}
              {isNavigationOpen ? <ApplicationVersion /> : null}
            </section>
          </motion.nav>
        </>
      ) : null}
      <MobileNavModal
        isOpen={isMobileNavOpen}
        setIsOpen={setMobileNavOpen}
        links={navItems}
        onLogOut={() => logOutMutation.mutate()}
      />
    </>
  );
};
