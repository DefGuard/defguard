import './style.scss';

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
            title: 'Settings',
            linkPath: '/admin/settings',
            icon: <SvgIconEdit />,
            allowedToView: ['admin'],
            enabled: true,
          }}
        />
      </div>
      <button className="log-out" onClick={() => onLogOut()}>
        <SvgIconNavLogout /> <span>Log out</span>
      </button>
      <Divider key="app-version-divider" />
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
    </Modal>
  );
};
