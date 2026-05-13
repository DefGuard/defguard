import { createFileRoute } from '@tanstack/react-router';
import { PostureChecksPage } from '../../../../pages/PostureChecksPage/PostureChecksPage';
import { getDevicePostureVersionMetadataQueryOptions } from '../../../../shared/query';

export const Route = createFileRoute('/_authorized/_default/acl/posture-checks')({
  loader: ({ context }) =>
    context.queryClient.fetchQuery(getDevicePostureVersionMetadataQueryOptions),
  component: PostureChecksPage,
});
