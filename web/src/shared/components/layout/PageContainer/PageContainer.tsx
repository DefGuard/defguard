import './style.scss';

import { ComponentPropsWithoutRef, forwardRef } from 'react';

import Navigation from '../../../../components/Navigation/Navigation';
import { useNavigationStore } from '../../../hooks/store/useNavigationStore';

/**
 * Standard layout for a page.
 *
 * Displays `BreadCrumbs` component above mobile resolution and renders `Navigation`.
 */
const PageContainer = forwardRef<
  HTMLDivElement,
  ComponentPropsWithoutRef<'div'>
>(({ children, className, ...rest }, ref) => {
  const isNavOpen = useNavigationStore((state) => state.isNavigationOpen);

  return (
    <div
      className={className ? `page-container ${className}` : 'page-container'}
      {...rest}
      ref={ref}
    >
      <Navigation />
      <div className={isNavOpen ? 'page-content nav-open' : 'page-content'}>
        {/*
            {breakpoint === 'desktop' && <BreadCrumbs />}
        */}
        {children}
      </div>
    </div>
  );
});

export default PageContainer;
