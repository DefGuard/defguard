import { createFileRoute } from '@tanstack/react-router';
import { AddPostureCheckWizardPage } from '../../../pages/AddPostureCheckWizardPage/AddPostureCheckWizardPage';

export const Route = createFileRoute('/_authorized/_wizard/add-posture-check')({
  component: AddPostureCheckWizardPage,
});
