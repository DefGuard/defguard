import './style.scss';

import clsx from 'clsx';
import { HTMLAttributes, PropsWithChildren } from 'react';

import { useNavigationStore } from '../../../../components/Navigation/hooks/useNavigationStore';

type Props = PropsWithChildren & HTMLAttributes<HTMLDivElement>;

export const PageLimiter = ({ children, className, ...divProps }: Props) => {
  const navOpen = useNavigationStore((s) => s.isOpen);

  return (
    <div
      className={clsx('page-limiter', className, {
        'nav-open': navOpen,
      })}
      {...divProps}
    >
      <div className="page-limited-content">{children}</div>
    </div>
  );
};
