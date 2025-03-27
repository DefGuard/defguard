import './style.scss';

import classNames from 'classnames';
import { useMatch } from 'react-router';
import { Link } from 'react-router-dom';
import { shallow } from 'zustand/shallow';

import { useUpgradeLicenseModal } from '../../../../shared/components/Layout/UpgradeLicenseModal/store';
import { UpgradeLicenseModalVariant } from '../../../../shared/components/Layout/UpgradeLicenseModal/types';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import { NavigationItem } from '../../types';

interface NavigationLinkProps {
  item: NavigationItem;
  callback?: () => void;
}

export const NavigationLink = ({ item, callback }: NavigationLinkProps) => {
  const openUpgradeLicenseModal = useUpgradeLicenseModal((s) => s.open, shallow);
  const enterpriseEnabled = useAppStore((s) => s.appInfo?.license_info.enterprise);
  const match = useMatch(item.linkPath);

  const cn = classNames(
    'navigation-link',
    {
      active: match,
    },
    item.className,
  );

  return (
    <Link
      replace
      to={item.linkPath}
      className={cn}
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
      <span>{item.title}</span>
      {match ? <div className="active-line" /> : null}
    </Link>
  );
};
