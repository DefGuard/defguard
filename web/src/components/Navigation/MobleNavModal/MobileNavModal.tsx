import './style.scss';

import { useI18nContext } from '../../../i18n/i18n-react';
import Divider from '../../../shared/components/layout/Divider/Divider';
import Modal from '../../../shared/components/layout/Modal/Modal';
import SvgDefguadNavLogo from '../../../shared/components/svg/DefguadNavLogo';
import SvgIconEdit from '../../../shared/components/svg/IconEditAlt';
import SvgIconHamburgerClose from '../../../shared/components/svg/IconHamburgerClose';
import SvgIconNavLogout from '../../../shared/components/svg/IconNavLogout';
import { ApplicationVersion } from '../ApplicationVersion/ApplicationVersion';
import { NavigationItem } from '../Navigation';
import { NavigationLink } from '../NavigationLink';

interface Props {
  isOpen: boolean;
  setIsOpen: (v: boolean) => void;
  links: NavigationItem[];
  onLogOut: () => void;
}

export const MobileNavModal = ({
  isOpen,
  setIsOpen,
  links,
  onLogOut,
}: Props) => {
  const { LL } = useI18nContext();
  return (
    <Modal
      className="mobile-nav"
      isOpen={isOpen}
      setIsOpen={setIsOpen}
      backdrop
    >
      <button className="close-mobile-nav" onClick={() => setIsOpen(false)}>
        <SvgIconHamburgerClose />
      </button>
      <div className="logo-container">
        <SvgDefguadNavLogo />
      </div>
      <div className="links">
        {links.map((link) => (
          <NavigationLink
            item={link}
            key={link.linkPath}
            callback={() => setIsOpen(false)}
          />
        ))}
      </div>
      <div className="links bottom">
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
      </div>
      <button className="log-out" onClick={() => onLogOut()}>
        <SvgIconNavLogout /> <span>{LL.navigation.bar.logOut()}</span>
      </button>
      <Divider key="app-version-divider" />
      <ApplicationVersion />
    </Modal>
  );
};
