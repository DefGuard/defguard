import { createFileRoute } from '@tanstack/react-router';
import { AddExternalOpenIdWizardPage } from '../../../pages/AddExternalOpenIdWizardPage/AddExternalOpenIdWizardPage';

export const Route = createFileRoute('/_authorized/_wizard/add-external-openid')({
  component: AddExternalOpenIdWizardPage,
});
