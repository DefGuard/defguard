import './shared/scss/styles.scss';
import './shared/defguard-ui/scss/index.scss';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import dayjs from 'dayjs';
import LocalizedFormat from 'dayjs/plugin/localizedFormat';
import utc from 'dayjs/plugin/utc';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { AppLoader } from './components/AppLoader';
import { TranslationProvider } from './shared/components/providers/TranslationProvider';

dayjs.extend(utc);
dayjs.extend(LocalizedFormat);

const queryClient = new QueryClient();
const root = createRoot(document.getElementById('root') as HTMLElement);
root.render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <TranslationProvider>
        <AppLoader />
      </TranslationProvider>
    </QueryClientProvider>
  </StrictMode>,
);
