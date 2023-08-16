import './style.scss';

import SvgIconHamburgerClose from '../../../../../shared/components/svg/IconHamburgerClose';
import { Modal } from '../../../../../shared/defguard-ui/components/Layout/modals/Modal/Modal';
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
