import './shared/scss/styles.scss';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { AppLoader } from './components/AppLoader';
import TypesafeI18n from './i18n/i18n-react';
import { Web3ContextProvider } from './shared/components/Web3/Web3ContextProvider';

const queryClient = new QueryClient();
const root = createRoot(document.getElementById('root') as HTMLElement);
root.render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <Web3ContextProvider>
        <TypesafeI18n locale="en">
          <AppLoader />
        </TypesafeI18n>
      </Web3ContextProvider>
    </QueryClientProvider>
  </StrictMode>,
);
