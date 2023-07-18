import './style.scss';

import { Modal } from '../../../../../shared/components/layout/Modal/Modal';
import SvgIconHamburgerClose from '../../../../../shared/components/svg/IconHamburgerClose';
import { useNavigationStore } from '../../../hooks/useNavigationStore';
import { NavigationItems } from '../../../types';
import { NavigationBar } from '../../NavigationBar/NavigationBar';

interface Props {
  navItems: NavigationItems;
  onLogout: () => void;
}

export const MobileNavModal = ({ navItems, onLogout }: Props) => {
  const setStore = useNavigationStore((state) => state.setState);
  const isOpen = useNavigationStore((state) => state.isOpen);
  return (
    <Modal
      className="mobile-nav"
      isOpen={isOpen}
      setIsOpen={(val) => setStore({ isOpen: val })}
      backdrop
    >
      <button className="close-mobile-nav" onClick={() => setStore({ isOpen: false })}>
        <SvgIconHamburgerClose />
      </button>
      <NavigationBar navItems={navItems} onLogout={onLogout} isOpen={true} />
    </Modal>
  );
};
