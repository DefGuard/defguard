import {
  autoUpdate,
  offset,
  shift,
  size,
  useClick,
  useDismiss,
  useFloating,
  useInteractions,
} from '@floating-ui/react';
import { Avatar } from '../../defguard-ui/components/Avatar/Avatar';
import { Divider } from '../../defguard-ui/components/Divider/Divider';
import './style.scss';
import { useNavigate } from '@tanstack/react-router';
import { useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../api/api';
import { IconButton } from '../../defguard-ui/components/IconButton/IconButton';
import { Menu } from '../../defguard-ui/components/Menu/Menu';
import type { MenuItemsGroup } from '../../defguard-ui/components/Menu/types';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../defguard-ui/types';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import { useApp } from '../../hooks/useApp';
import { useAuth } from '../../hooks/useAuth';

type Props = {
  title: string;
  navOpen: boolean;
};

export const PageTopBar = ({ title, navOpen }: Props) => {
  const isAdmin = useAuth((s) => s.isAdmin);
  return (
    <div className="page-top-bar">
      {!navOpen && isAdmin && (
        <>
          <IconButton
            onClick={() => {
              useApp.setState({
                navigationOpen: true,
              });
            }}
            icon="hamburger"
          />
          <SizedBox height={1} width={ThemeSpacing.Xl} />
        </>
      )}
      <p className="page-title">{title}</p>
      <div className="right">
        <Divider orientation="vertical" />
        <ProfileMenu />
      </div>
    </div>
  );
};

const ProfileMenu = () => {
  const navigate = useNavigate();
  const resetAuth = useAuth((s) => s.reset);
  const user = useAuth((s) => s.user);

  const menuItems = useMemo(() => {
    if (!isPresent(user)) return [];
    const res: MenuItemsGroup[] = [
      {
        items: [
          {
            text: 'Profile',
            icon: 'profile',
            testId: 'profile',
            onClick: () => {
              navigate({
                to: '/user/$username',
                params: {
                  username: user.username,
                },
              });
            },
          },
          {
            text: m.controls_logout(),
            icon: 'logout',
            testId: 'logout',
            onClick: () => {
              api.auth.logout().then(() => {
                resetAuth();
              });
            },
          },
        ],
      },
    ];
    return res;
  }, [resetAuth, navigate, user?.username, user]);

  const [isOpen, setOpen] = useState(false);

  const { refs, context, floatingStyles } = useFloating({
    placement: 'bottom-end',
    whileElementsMounted: autoUpdate,
    onOpenChange: setOpen,
    open: isOpen,
    middleware: [
      offset(4),
      shift(),
      size({
        apply({ rects, elements, availableHeight }) {
          const refWidth = `${rects.reference.width}px`;
          elements.floating.style.minWidth = refWidth;
          elements.floating.style.maxHeight = `${availableHeight - 10}px`;
        },
      }),
    ],
  });

  const click = useClick(context, {
    toggle: true,
  });

  const dismiss = useDismiss(context, {
    ancestorScroll: true,
    escapeKey: true,
    outsidePress: (event) => !(event.target as HTMLElement).closest('.menu'),
  });

  const { getFloatingProps, getReferenceProps } = useInteractions([click, dismiss]);

  return (
    <>
      <Avatar data-testid='avatar-icon' ref={refs.setReference} {...getReferenceProps()} />
      {isOpen && (
        <Menu
          ref={refs.setFloating}
          {...getFloatingProps()}
          style={floatingStyles}
          itemGroups={menuItems}
        />
      )}
    </>
  );
};
