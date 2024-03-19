import { useNavigationStore } from '../../hooks/useNavigationStore';
import { NavigationItems } from '../../types';
import { NavigationBar } from '../NavigationBar/NavigationBar';
import { NavigationCollapse } from './NavigationCollapse/NavigationCollapse';

type Props = {
  navItems: NavigationItems;
  onLogout: () => void;
};

export const NavigationDesktop = ({ navItems, onLogout }: Props) => {
  const isOpen = useNavigationStore((state) => state.isOpen);
  return (
    <>
      <NavigationBar navItems={navItems} onLogout={onLogout} isOpen={isOpen} />
      <NavigationCollapse />
    </>
  );
};
