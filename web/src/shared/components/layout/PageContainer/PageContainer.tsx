import './style.scss';

import classNames from 'classnames';
import { ComponentPropsWithoutRef, forwardRef, useMemo } from 'react';

import { Navigation } from '../../../../components/Navigation/Navigation';
import { useNavigationStore } from '../../../hooks/store/useNavigationStore';

export const PageContainer = forwardRef<HTMLDivElement, ComponentPropsWithoutRef<'div'>>(
  ({ children, className, ...rest }, ref) => {
    const isNavOpen = useNavigationStore((state) => state.isNavigationOpen);
    const cn = useMemo(() => classNames('page-container', className), [className]);
    const contentCn = useMemo(
      () => classNames('page-content', { 'nav-open': isNavOpen }),
      [isNavOpen]
    );
    return (
      <div {...rest} className={cn} ref={ref}>
        <Navigation />
        <div className={contentCn}>{children}</div>
      </div>
    );
  }
);
