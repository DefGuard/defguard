import './i18n';
import './shared/scss/styles.scss';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { lazy, StrictMode, Suspense } from 'react';
import { createRoot } from 'react-dom/client';

import LoaderPage from './pages/loader/LoaderPage';
import { Web3ContextProvider } from './shared/components/Web3/Web3ContextProvider';

const App = lazy(() => import('./components/App/App'));

const queryClient = new QueryClient();
const root = createRoot(document.getElementById('root') as HTMLElement);
root.render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <Web3ContextProvider>
        <Suspense fallback={<LoaderPage />}>
          <App />
        </Suspense>
      </Web3ContextProvider>
    </QueryClientProvider>
  </StrictMode>
);
