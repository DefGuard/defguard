import { useState } from 'react';
import { m } from '../../../paraglide/messages';
import { Icon } from '../../defguard-ui/components/Icon';
import type { IconKindValue } from '../../defguard-ui/components/Icon/icon-types';
import { IconButton } from '../../defguard-ui/components/IconButton/IconButton';
import { useApp } from '../../hooks/useApp';
import { useAuth } from '../../hooks/useAuth';
import { NavLogo } from './assets/NavLogo';
import './style.scss';
import { Link, type LinkProps } from '@tanstack/react-router';
import { Fold } from '../../defguard-ui/components/Fold/Fold';

interface NavGroupProps {
  id: string;
  label: string;
  items: NavItemProps[];
}

interface NavItemProps {
  id: string;
  label: string;
  icon: IconKindValue;
  link: LinkProps['to'];
}

const navigationConfig: NavGroupProps[] = [
  {
    id: 'identity',
    label: m.cmp_nav_group_identity(),
    items: [
      {
        id: 'users',
        icon: 'users',
        label: m.cmp_nav_item_users(),
        link: '/users',
      },
      {
        id: 'groups',
        icon: 'groups',
        label: m.cmp_nav_item_groups(),
        link: '/groups',
      },
    ],
  },
  {
    id: 'integrations',
    label: m.cmp_nav_group_integrations(),
    items: [
      {
        id: 'network_devices',
        icon: 'devices',
        label: m.cmp_nav_item_network_devices(),
        link: '/network-devices',
      },
      {
        id: 'openid',
        icon: 'openid',
        label: m.cmp_nav_item_openid(),
        link: '/openid',
      },
      {
        id: 'webhooks',
        icon: 'webhooks',
        label: m.cmp_nav_item_webhooks(),
        link: '/webhooks',
      },
    ],
  },
];

export const Navigation = () => {
  const isAdmin = useAuth((s) => s.isAdmin);
  const isOpen = useApp((s) => s.navigationOpen);

  if (!isAdmin || !isOpen) return null;
  return (
    <div className="navigation">
      <div className="top">
        <NavLogo />
        <div className="control">
          <IconButton
            icon="hamburger"
            onClick={() => {
              useApp.setState({
                navigationOpen: false,
              });
            }}
          />
        </div>
      </div>
      <div className="groups">
        {navigationConfig.map((group) => (
          <NavGroup key={group.id} {...group} />
        ))}
      </div>
      <div className="bottom"></div>
    </div>
  );
};

const NavGroup = ({ items, label }: NavGroupProps) => {
  const [isOpen, setIsOpen] = useState(true);
  return (
    <div className="nav-group">
      <div
        className="track"
        onClick={() => {
          setIsOpen((s) => !s);
        }}
      >
        <Icon icon="arrow-small" rotationDirection={isOpen ? 'down' : 'right'} />
        <p>{label}</p>
      </div>
      <Fold open={isOpen}>
        <div className="items">
          {items.map((item) => (
            <NavItem key={item.id} {...item} />
          ))}
        </div>
      </Fold>
    </div>
  );
};

const NavItem = ({ icon, link, label }: NavItemProps) => {
  return (
    <Link to={link} className="nav-item">
      <Icon icon={icon} />
      <span>{label}</span>
    </Link>
  );
};
