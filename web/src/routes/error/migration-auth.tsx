import { createFileRoute, redirect } from '@tanstack/react-router';
import { ErrorMigrationInProgressPage } from '../../pages/ErrorMigrationInProgressPage/ErrorMigrationInProgressPage';
import { getSessionInfoQueryOptions } from '../../shared/query';

export const Route = createFileRoute('/_authorized/error/migration-auth')({
  beforeLoad: async ({ context }) => {
    const sessionInfo = (await context.queryClient.fetchQuery(getSessionInfoQueryOptions))
      .data;
    if (!sessionInfo.authorized) {
      throw redirect({ to: '/auth', replace: true });
    }
  },
  component: ErrorMigrationInProgressPage,
});
