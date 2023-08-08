import './shared/scss/styles.scss';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { AppLoader } from './components/AppLoader';
import TypesafeI18n from './i18n/i18n-react';

const queryClient = new QueryClient();
const root = createRoot(document.getElementById('root') as HTMLElement);
root.render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <TypesafeI18n locale="en">
        <AppLoader />
      </TypesafeI18n>
    </QueryClientProvider>
  </StrictMode>,
);
