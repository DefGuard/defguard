import './style.scss';

import classNames from 'classnames';
import { useMemo } from 'react';

import { useI18nContext } from '../../../../i18n/i18n-react';
import SvgDefguadNavLogoCollapsed from '../../../../shared/components/svg/DefguadNavLogoCollapsed';
import SvgIconNavLogout from '../../../../shared/components/svg/IconNavLogout';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import { NavigationItems } from '../../types';
import { ApplicationVersion } from '../ApplicationVersion/ApplicationVersion';
import { NavigationLink } from '../NavigationLink/NavigationLink';

type Props = {
  navItems: NavigationItems;
  onLogout: () => void;
  isOpen: boolean;
};

export const NavigationBar = ({ navItems, onLogout, isOpen }: Props) => {
  const settings = useAppStore((state) => state.settings);
  const { LL } = useI18nContext();

  const cn = useMemo(
    () =>
      classNames('nav-bar', {
        open: isOpen,
      }),
    [isOpen]
  );

  return (
    <nav className={cn}>
      <div className="logo-container">
        {isOpen && <img src={settings?.nav_logo_url} />}
        {!isOpen && <SvgDefguadNavLogoCollapsed />}
      </div>
      <div
        className="links"
        style={{
          minHeight: (navItems.middle.length + navItems.bottom.length + 1) * 58,
        }}
      >
        <div className="middle">
          {navItems.middle.map((item) => (
            <NavigationLink key={item.linkPath} item={item} />
          ))}
        </div>
        <div className="bottom">
          {navItems.bottom.map((item) => (
            <NavigationLink key={item.linkPath} item={item} />
          ))}
          <button data-testid="logout" className="log-out" onClick={onLogout}>
            <SvgIconNavLogout />
            <span>{LL.navigation.bar.logOut()}</span>
          </button>
        </div>
      </div>
      <ApplicationVersion isOpen={isOpen} />
    </nav>
  );
};
