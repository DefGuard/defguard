import './day';

import { QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import { useEffect } from 'react';
import { AppThemeProvider } from '../shared/providers/AppThemeProvider';
import { queryClient } from './query';
import { router } from './router';

export const App = () => {
  useEffect(() => {
    // Safety net: if a modal is open and the user navigates to a route that
    // doesn't render it, ModalFoundation's unmount cleanup can fail to fire,
    // leaving body scroll locked with no modal actually on screen.
    const modalsRoot = document.getElementById('modals-root');
    const rootElement = document.getElementById('root');
    if (!modalsRoot || !rootElement) return;

    const observer = new MutationObserver(() => {
      if (
        modalsRoot.childElementCount === 0 &&
        rootElement.style.overflowY === 'hidden'
      ) {
        rootElement.style.overflowY = 'auto';
      }
    });

    observer.observe(modalsRoot, { childList: true });

    return () => {
      observer.disconnect();
    };
  }, []);

  return (
    <AppThemeProvider>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </AppThemeProvider>
  );
};
