import { createFileRoute } from '@tanstack/react-router';
import { AddPostureCheckWizardPage } from '../../../pages/AddPostureCheckWizardPage/AddPostureCheckWizardPage';
import { getDevicePostureVersionMetadataQueryOptions } from '../../../shared/query';

export const Route = createFileRoute('/_authorized/_wizard/add-posture-check')({
  loader: ({ context }) =>
    context.queryClient.fetchQuery(getDevicePostureVersionMetadataQueryOptions),
  component: AddPostureCheckWizardPage,
});
