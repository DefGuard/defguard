import './style.scss';
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
import { useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useMemo, useState } from 'react';
import { m } from '../../../../../paraglide/messages';
import api from '../../../../api/api';
import { Avatar } from '../../../../defguard-ui/components/Avatar/Avatar';
import { Icon } from '../../../../defguard-ui/components/Icon';
import { Menu } from '../../../../defguard-ui/components/Menu/Menu';
import type { MenuItemsGroup } from '../../../../defguard-ui/components/Menu/types';
import { Direction } from '../../../../defguard-ui/types';
import { isPresent } from '../../../../defguard-ui/utils/isPresent';
import { useAuth } from '../../../../hooks/useAuth';
import { getUserMeQueryOptions } from '../../../../query';
import { TopBarElementSkeleton } from '../../TopBarElementSkeleton';

export const TopBarProfile = () => {
  const queryClient = useQueryClient();
  const navigate = useNavigate();
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
                queryClient.invalidateQueries({
                  queryKey: getUserMeQueryOptions.queryKey,
                });
                useAuth.getState().reset();
                setTimeout(() => {
                  navigate({ to: '/auth/login', replace: true });
                }, 100);
              });
            },
          },
        ],
      },
    ];
    return res;
  }, [navigate, user, queryClient]);

  const [isOpen, setOpen] = useState(false);

  const { refs, context, floatingStyles } = useFloating({
    placement: 'bottom-end',
    whileElementsMounted: autoUpdate,
    onOpenChange: setOpen,
    open: isOpen,
    middleware: [
      offset(12),
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

  if (!user) return <TopBarElementSkeleton />;
  return (
    <>
      <div id="top-bar-profile" ref={refs.setReference} {...getReferenceProps()}>
        <Avatar
          variant="initials"
          size="default"
          firstName={user.first_name}
          lastName={user.last_name}
        />
        <p>{user.email}</p>
        <Icon icon="arrow-small" rotationDirection={Direction.DOWN} />
      </div>
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
