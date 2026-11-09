import { createFileRoute } from '@tanstack/react-router';
import { AddPostureCheckWizardPage } from '../../../pages/AddPostureCheckWizardPage/AddPostureCheckWizardPage';
import {
  getDeviceAttestationStatsQueryOptions,
  getDevicePostureVersionMetadataQueryOptions,
} from '../../../shared/query';

export const Route = createFileRoute('/_authorized/_wizard/add-posture-check')({
  loader: ({ context }) =>
    Promise.all([
      context.queryClient.fetchQuery(getDevicePostureVersionMetadataQueryOptions),
      context.queryClient.fetchQuery(getDeviceAttestationStatsQueryOptions),
    ]),
  component: AddPostureCheckWizardPage,
});
