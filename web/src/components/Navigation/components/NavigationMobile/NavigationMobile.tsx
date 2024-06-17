import './style.scss';

import { useMemo } from 'react';
import { useLocation } from 'react-router';

import { useI18nContext } from '../../../../i18n/i18n-react';
import SvgDefguardNavLogoCollapsed from '../../../../shared/components/svg/DefguardNavLogoCollapsed';
import SvgIconNavHamburger from '../../../../shared/components/svg/IconNavHamburger';
import { useNavigationStore } from '../../hooks/useNavigationStore';
import { NavigationItems } from '../../types';
import { MobileNavModal } from './MobileNavModal/MobileNavModal';

type Props = {
  onLogout: () => void;
  navItems: NavigationItems;
};

export const NavigationMobile = ({ navItems, onLogout }: Props) => {
  const { LL } = useI18nContext();
  const { pathname } = useLocation();
  const setStore = useNavigationStore((state) => state.setState);

  const titleMap = useMemo(
    () => [
      {
        path: '/admin/settings',
        title: LL.navigation.mobileTitles.settings(),
      },
      {
        path: '/admin/users',
        title: LL.navigation.mobileTitles.users(),
      },
      {
        path: '/admin/user',
        title: LL.navigation.mobileTitles.user(),
      },
      {
        path: '/admin/me',
        title: LL.navigation.mobileTitles.user(),
      },
      {
        path: '/admin/provisioners',
        title: LL.navigation.mobileTitles.provisioners(),
      },
      {
        path: '/admin/webhooks',
        title: LL.navigation.mobileTitles.webhooks(),
      },
      {
        path: '/admin/wizard',
        title: LL.navigation.mobileTitles.wizard(),
      },
      {
        path: '/admin/network',
        title: LL.navigation.mobileTitles.networkSettings(),
      },
      {
        path: '/admin/overview',
        title: LL.navigation.mobileTitles.overview(),
      },
      {
        path: '/admin/enrollment',
        title: LL.navigation.mobileTitles.enrollment(),
      },
      {
        path: '/admin/openid',
        title: LL.navigation.mobileTitles.openId(),
      },
    ],
    [LL.navigation.mobileTitles],
  );

  const getPageTitle = useMemo(() => {
    for (const item of titleMap) {
      if (pathname.includes(item.path)) {
        return item.title;
      }
    }
    return '';
  }, [pathname, titleMap]);

  return (
    <>
      <nav className="nav-mobile">
        <SvgDefguardNavLogoCollapsed />
        <p className="page-title">{getPageTitle}</p>
        <button className="hamburger" onClick={() => setStore({ isOpen: true })}>
          <SvgIconNavHamburger />
        </button>
      </nav>
      <MobileNavModal navItems={navItems} onLogout={onLogout} />
    </>
  );
};
