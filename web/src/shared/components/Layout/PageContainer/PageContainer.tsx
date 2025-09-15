import './style.scss';

import clsx from 'clsx';
import type { ComponentProps } from 'react';
import { useNavigationStore } from '../../../../components/Navigation/hooks/useNavigationStore';

type Props = {
  withDefaultPadding?: boolean;
} & ComponentProps<'div'>;

export const PageContainer = ({
  children,
  className,
  ref,
  withDefaultPadding = false,
  ...rest
}: Props) => {
  const isNavOpen = useNavigationStore((state) => state.isOpen);
  return (
    <div {...rest} className={clsx('page-container', className)}>
      <div
        className={clsx('page-content', {
          'nav-open': isNavOpen,
          'default-padding': withDefaultPadding,
        })}
      >
        {children}
      </div>
    </div>
  );
};
