import './shared/scss/styles.scss';
import './shared/defguard-ui/scss/index.scss';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import dayjs from 'dayjs';
import LocalizedFormat from 'dayjs/plugin/localizedFormat';
import utc from 'dayjs/plugin/utc';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { AppLoader } from './components/AppLoader';
import { LocaleProvider } from './components/LocaleProvider';
import { ApiProvider } from './shared/hooks/api/provider';

dayjs.extend(utc);
dayjs.extend(LocalizedFormat);

const queryClient = new QueryClient();
const root = createRoot(document.getElementById('root') as HTMLElement);
root.render(
  <StrictMode>
    <LocaleProvider>
      <ApiProvider>
        <QueryClientProvider client={queryClient}>
          <AppLoader />
        </QueryClientProvider>
      </ApiProvider>
    </LocaleProvider>
  </StrictMode>,
);
