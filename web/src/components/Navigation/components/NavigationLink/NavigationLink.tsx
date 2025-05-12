import './style.scss';

import clsx from 'clsx';
import { useMatch } from 'react-router';
import { Link } from 'react-router-dom';
import { shallow } from 'zustand/shallow';

import { useUpgradeLicenseModal } from '../../../../shared/components/Layout/UpgradeLicenseModal/store';
import { UpgradeLicenseModalVariant } from '../../../../shared/components/Layout/UpgradeLicenseModal/types';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import { useNavigationStore } from '../../hooks/useNavigationStore';
import { NavigationItem } from '../../types';

interface NavigationLinkProps {
  item: NavigationItem;
  callback?: () => void;
}

export const NavigationLink = ({ item, callback }: NavigationLinkProps) => {
  const isOpen = useNavigationStore((s) => s.isOpen);
  const openUpgradeLicenseModal = useUpgradeLicenseModal((s) => s.open, shallow);
  const enterpriseEnabled = useAppStore((s) => s.appInfo?.license_info.enterprise);
  const match = useMatch(item.linkPath);

  return (
    <Link
      replace
      to={item.linkPath}
      className={clsx(
        'navigation-link',
        {
          active: match,
          compact: !isOpen,
        },
        item.className,
      )}
      onClick={(event) => {
        if (item.enterpriseOnly && !enterpriseEnabled) {
          event.preventDefault();
          openUpgradeLicenseModal({
            modalVariant: UpgradeLicenseModalVariant.ENTERPRISE_NOTICE,
          });
        }
        if (callback) {
          callback();
        }
        if (item.onClick) {
          item.onClick();
        }
      }}
    >
      {item.icon}
      {isOpen && <span>{item.title}</span>}
      {match ? <div className="active-line" /> : null}
    </Link>
  );
};
