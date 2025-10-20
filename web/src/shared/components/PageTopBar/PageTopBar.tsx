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
import { Menu } from '../../defguard-ui/components/Menu/Menu';
import type { MenuItemsGroup } from '../../defguard-ui/components/Menu/types';
import { useAuth } from '../../hooks/useAuth';

type Props = {
  title: string;
};

export const PageTopBar = ({ title }: Props) => {
  return (
    <div className="page-top-bar">
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

  const menuItems = useMemo(() => {
    const res: MenuItemsGroup[] = [
      {
        items: [
          {
            text: m.controls_logout(),
            icon: 'logout',
            onClick: () => {
              api.auth.logout().then(() => {
                resetAuth();
                navigate({
                  to: '/auth/login',
                  replace: true,
                });
              });
            },
          },
        ],
      },
    ];
    return res;
  }, [navigate, resetAuth]);

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
      <Avatar ref={refs.setReference} {...getReferenceProps()} />
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
