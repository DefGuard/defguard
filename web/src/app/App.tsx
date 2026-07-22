import './day';

import { QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import { useEffect } from 'react';
import { AppThemeProvider } from '../shared/providers/AppThemeProvider';
import { queryClient } from './query';
import { router } from './router';

export const App = () => {
  useEffect(() => {
    const modalsRoot = document.getElementById('modals-root');
    const rootElement = document.getElementById('root');
    if (!modalsRoot || !rootElement) return;

    // When a modal opens, ModalFoundation locks background scrolling by hiding
    // #root's vertical scrollbar. On platforms/settings where that scrollbar
    // occupies layout width (Windows, or macOS/Safari with "always show
    // scrollbars") its removal widens the content area and shifts the page
    // sideways. We reserve that width back only while the lock is active, so no
    // permanent gap remains and overlay-scrollbar setups (which reserve 0) are
    // unaffected. Only compensate when the page was actually scrollable.
    const supportsScrollbarGutter = CSS.supports('scrollbar-gutter', 'stable');
    const isScrollLocked = () => rootElement.style.overflowY === 'hidden';

    // Fallback for engines without scrollbar-gutter.
    let scrollbarWidth = 0;
    if (!supportsScrollbarGutter) {
      const probe = document.createElement('div');
      probe.style.cssText =
        'position:absolute;top:-9999px;width:100px;height:100px;overflow:scroll';
      document.body.appendChild(probe);
      scrollbarWidth = probe.offsetWidth - probe.clientWidth;
      probe.remove();
    }

    const syncScrollbarCompensation = () => {
      const locked = isScrollLocked();
      const scrollable = rootElement.scrollHeight > rootElement.clientHeight;
      const compensate = locked && scrollable;
      if (supportsScrollbarGutter) {
        const desired = compensate ? 'stable' : '';
        if (rootElement.style.scrollbarGutter !== desired) {
          rootElement.style.scrollbarGutter = desired;
        }
      } else {
        const desired = compensate && scrollbarWidth > 0 ? `${scrollbarWidth}px` : '';
        if (rootElement.style.paddingRight !== desired) {
          rootElement.style.paddingRight = desired;
        }
      }
    };

    const rootObserver = new MutationObserver(syncScrollbarCompensation);
    rootObserver.observe(rootElement, {
      attributes: true,
      attributeFilter: ['style'],
    });

    const modalsObserver = new MutationObserver(() => {
      if (modalsRoot.childElementCount === 0 && isScrollLocked()) {
        rootElement.style.overflowY = 'auto';
      }
    });

    modalsObserver.observe(modalsRoot, { childList: true });

    return () => {
      rootObserver.disconnect();
      modalsObserver.disconnect();
      rootElement.style.paddingRight = '';
      rootElement.style.scrollbarGutter = '';
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
