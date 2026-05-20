import { createFileRoute } from '@tanstack/react-router';
import { EditPostureCheckPage } from '../../../../../../pages/EditPostureCheckPage/EditPostureCheckPage';
import {
  getDevicePostureQueryOptions,
  getDevicePostureVersionMetadataQueryOptions,
  getLocationsQueryOptions,
} from '../../../../../../shared/query';

export const Route = createFileRoute(
  '/_authorized/_default/acl/posture-checks/$postureCheckId/edit',
)({
  loader: async ({ context, params }) => {
    const postureCheckId = parseInt(params.postureCheckId, 10);

    await Promise.all([
      context.queryClient.fetchQuery(getDevicePostureQueryOptions(postureCheckId)),
      context.queryClient.fetchQuery(getDevicePostureVersionMetadataQueryOptions),
      context.queryClient.fetchQuery(getLocationsQueryOptions),
    ]);
  },
  component: EditPostureCheckPage,
});
