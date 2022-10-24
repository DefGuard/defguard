import './i18n';
import './shared/scss/styles.scss';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { lazy, StrictMode, Suspense } from 'react';
import { createRoot } from 'react-dom/client';

import LoaderPage from './pages/loader/LoaderPage';

const App = lazy(() => import('./components/App/App'));

const queryClient = new QueryClient();
const root = createRoot(document.getElementById('root') as HTMLElement);
root.render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <Suspense fallback={<LoaderPage />}>
        <App />
      </Suspense>
    </QueryClientProvider>
  </StrictMode>
);
