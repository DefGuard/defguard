import './style.scss';

import clsx from 'clsx';
import { type ComponentProps, useEffect } from 'react';
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
  useEffect(() => {
    console.log({ withDefaultPadding });
  }, [withDefaultPadding]);
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
