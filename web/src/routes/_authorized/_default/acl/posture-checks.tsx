import { createFileRoute, Outlet, useRouterState } from '@tanstack/react-router';
import { PostureChecksPage } from '../../../../pages/PostureChecksPage/PostureChecksPage';
import { isPostureChecksListPath } from '../../../../pages/PostureChecksPage/route';
import { getDevicePostureVersionMetadataQueryOptions } from '../../../../shared/query';

export const Route = createFileRoute('/_authorized/_default/acl/posture-checks')({
  loader: ({ context }) =>
    context.queryClient.fetchQuery(getDevicePostureVersionMetadataQueryOptions),
  component: RouteComponent,
});

function RouteComponent() {
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });

  if (isPostureChecksListPath(pathname)) {
    return <PostureChecksPage />;
  }

  return <Outlet />;
}
