import './day';

import { QueryClientProvider } from '@tanstack/react-query';
import { RouterProvider } from '@tanstack/react-router';
import { AppThemeProvider } from '../shared/providers/AppThemeProvider';
import { queryClient } from './query';
import { router } from './router';

export const App = () => {
  return (
    <AppThemeProvider>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </AppThemeProvider>
  );
};
