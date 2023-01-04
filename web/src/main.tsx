import './shared/scss/styles.scss';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { lazy, StrictMode, Suspense } from 'react';
import { createRoot } from 'react-dom/client';

import LoaderPage from './pages/loader/LoaderPage';
import { Web3ContextProvider } from './shared/components/Web3/Web3ContextProvider';

const App = lazy(() => import('./components/App/App'));
const TypesafeI18n = lazy(() => import('./i18n/i18n-react'));

const queryClient = new QueryClient();
const root = createRoot(document.getElementById('root') as HTMLElement);
root.render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <Web3ContextProvider>
        <Suspense fallback={<LoaderPage />}>
          <TypesafeI18n locale="en">
            <Suspense fallback={<LoaderPage />}>
              <App />
            </Suspense>
          </TypesafeI18n>
        </Suspense>
      </Web3ContextProvider>
    </QueryClientProvider>
  </StrictMode>
);
