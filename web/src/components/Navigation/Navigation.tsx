import './style.scss';

import { useMutation } from '@tanstack/react-query';
import { AnimatePresence, motion } from 'framer-motion';
import React, { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useLocation } from 'react-router-dom';
import useBreakpoint from 'use-breakpoint';
import shallow from 'zustand/shallow';

import Divider from '../../shared/components/layout/Divider/Divider';
import IconButton from '../../shared/components/layout/IconButton/IconButton';
import SvgDefguadNavLogo from '../../shared/components/svg/DefguadNavLogo';
import SvgDefguadNavLogoCollapsed from '../../shared/components/svg/DefguadNavLogoCollapsed';
import SvgIconArrowDoubleGrayLeft from '../../shared/components/svg/IconArrowDoubleGrayLeft';
import SvgIconEdit from '../../shared/components/svg/IconEditAlt';
import SvgIconHamburgerMenu from '../../shared/components/svg/IconHamburgerMenu';
import SvgIconNavLogout from '../../shared/components/svg/IconNavLogout';
import SvgIconNavOpenId from '../../shared/components/svg/IconNavOpenid';
import SvgIconNavOverview from '../../shared/components/svg/IconNavOverview';
import SvgIconNavProfile from '../../shared/components/svg/IconNavProfile';
import SvgIconNavSettings from '../../shared/components/svg/IconNavSettings';
import SvgIconNavUsers from '../../shared/components/svg/IconNavUsers';
import { deviceBreakpoints } from '../../shared/constants';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import { useNavigationStore } from '../../shared/hooks/store/useNavigationStore';
import useApi from '../../shared/hooks/useApi';
import ApplicationVersion from './ApplicationVersion/ApplicationVersion';
import MobileNavModal from './MobleNavModal/MobileNavModal';
import NavigationLink from './NavigationLink';

export interface NavigationItem {
  title: string;
  linkPath: string;
  icon?: React.ReactNode;
  allowedToView?: string[];
  enabled: boolean | undefined;
}

const Navigation = () => {
  const { t } = useTranslation('en');
  const [currentUser, storeLogOut] = useAuthStore(
    (state) => [state.user, state.logOut],
    shallow
  );
  const [isMobileNavOpen, setMobileNavOpen] = useState(false);
  const [isNavigationOpen, setNavigationOpen, navigationUser] =
    useNavigationStore(
      (state) => [state.isNavigationOpen, state.setNavigationOpen, state.user],
      shallow
    );
  const {
    auth: { logout },
  } = useApi();
  const logOutMutation = useMutation(logout, {
    onSuccess: () => {
      storeLogOut();
    },
  });

  const settings = useAppStore((state) => state.settings);
  const { breakpoint } = useBreakpoint(deviceBreakpoints);

  const { pathname } = useLocation();

  const getPageTitle = useMemo(() => {
    if (pathname.includes('/me')) {
      return 'My profile';
    }
    if (pathname === '/admin/users' || pathname === '/admin/users/') {
      return 'Users';
    }
    if (pathname.includes('/admin/users/') && !pathname.includes('/edit')) {
      if (
        navigationUser &&
        navigationUser.first_name &&
        navigationUser.last_name
      ) {
        return `${navigationUser.first_name} ${navigationUser.last_name}`;
      }
    }
    if (
      pathname.includes('/admin/users') &&
      pathname.includes('/edit') &&
      navigationUser
    ) {
      return `Edit ${navigationUser.username}`;
    }
    if (pathname.includes('/admin/provisioners')) {
      return 'Provisioners';
    }
    if (pathname.includes('/admin/webhooks')) {
      return 'Webhooks';
    }
    if (pathname.includes('/admin/openid')) {
      return 'OpenID Apps';
    }
    if (pathname.includes('/admin/overview')) {
      return 'Network overview';
    }
    return '';
  }, [pathname, navigationUser]);

  const navItems: NavigationItem[] = useMemo(() => {
    let base: NavigationItem[] = [
      {
        title: 'Overview',
        linkPath: '/admin/overview',
        icon: <SvgIconNavOverview />,
        allowedToView: ['admin'],
        enabled: settings?.wireguard_enabled,
      },
      {
        title: t('navigation.template.links.users'),
        linkPath: '/admin/users',
        icon: <SvgIconNavUsers />,
        allowedToView: ['admin'],
        enabled: true,
      },
      {
        title: t('navigation.template.links.Provisioners'),
        linkPath: '/admin/provisioners',
        icon: <SvgIconNavOverview />,
        allowedToView: ['admin'],
        enabled: settings?.worker_enabled,
      },
      {
        title: t('navigation.template.links.Webhooks'),
        linkPath: '/admin/webhooks',
        icon: <SvgIconNavSettings />,
        allowedToView: ['admin'],
        enabled: settings?.webhooks_enabled,
      },
      {
        title: t('navigation.template.links.OpenIDApps'),
        linkPath: '/admin/openid',
        icon: <SvgIconNavOpenId />,
        allowedToView: ['admin'],
        enabled: settings?.openid_enabled,
      },
      {
        title: t('navigation.template.links.myProfile'),
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
    base = base.filter((item) => {
      if (item.enabled) {
        return true;
      } else {
        return false;
      }
    });
    return base;
  }, [currentUser, t, settings]);

  return (
    <>
      {breakpoint !== 'desktop' ? (
        <nav className="nav-mobile">
          <SvgDefguadNavLogoCollapsed />
          <p className="page-title">{getPageTitle}</p>
          <IconButton
            className="hamburger-button"
            onClick={() => setMobileNavOpen(true)}
          >
            <SvgIconHamburgerMenu />
          </IconButton>
        </nav>
      ) : null}
      {breakpoint === 'desktop' ? (
        <>
          <button
            onClick={() => setNavigationOpen(!isNavigationOpen)}
            className={
              'nav-control-button' + (isNavigationOpen ? '' : ' collapsed')
            }
          >
            <SvgIconArrowDoubleGrayLeft />
          </button>
          <motion.nav
            className={'nav-container ' + (isNavigationOpen ? 'visible' : '')}
            layout
          >
            <section className="logo-container">
              <SvgDefguadNavLogo /> <SvgDefguadNavLogoCollapsed />
            </section>
            <span className="divider"></span>
            <section className="links">
              {/* <NavLink
            to="/users"
            className={({ isActive }) => (isActive ? 'active' : '')}
          >
            <SvgIconNavOverview />
            <span>{t('navigation.template.links.overview')}</span>
          </NavLink>
          <NavLink
            to="/users"
            className={({ isActive }) => (isActive ? 'active' : '')}
          >
            <SvgIconNavUsers />
            <span>{t('navigation.template.links.users')}</span>
          </NavLink>
          <NavLink
            to="/users"
            className={({ isActive }) => (isActive ? 'active' : '')}
          >
            <SvgIconNavLocations />
            <span>{t('navigation.template.links.locations')}</span>
          </NavLink>
          <NavLink
            to="/users"
            className={({ isActive }) => (isActive ? 'active' : '')}
          >
            <SvgIconNavSettings />
            <span>{t('navigation.template.links.settings')}</span>
          </NavLink> */}
              {/* <a className="">
            <SvgIconNavOverview />
            <span>{t('navigation.template.links.overview')}</span>
          </a> */}
              {navItems.map((item) => (
                <NavigationLink key={item.linkPath} item={item} />
              ))}
              {/* <a className="">
            <SvgIconNavLocations />
            <span>{t('navigation.template.links.locations')}</span>
          </a>
          <a className="">
            <SvgIconNavSettings />
            <span>{t('navigation.template.links.settings')}</span>
          </a> */}
            </section>
            <motion.section className="bottom">
              <NavigationLink
                key={'/admin/settings'}
                item={{
                  title: t('navigation.template.links.settings'),
                  linkPath: '/admin/settings',
                  icon: <SvgIconEdit />,
                  allowedToView: ['admin'],
                  enabled: true,
                }}
              />
              <button
                className="log-out"
                onClick={() => logOutMutation.mutate()}
              >
                <SvgIconNavLogout />{' '}
                <span>{t('navigation.template.logOut')}</span>
              </button>
              <AnimatePresence>
                {isNavigationOpen ? (
                  <Divider key="app-version-divider" />
                ) : null}
                {isNavigationOpen ? (
                  <ApplicationVersion
                    key="app-version"
                    initial="hidden"
                    animate="show"
                    exit="hidden"
                    variants={{
                      hidden: {
                        opacity: 0,
                        transition: {
                          duration: 0.0,
                        },
                      },
                      show: {
                        opacity: 1,
                        transition: {
                          delay: 0.25,
                        },
                      },
                    }}
                    layout
                  />
                ) : null}
              </AnimatePresence>
            </motion.section>
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

export default Navigation;
