import { createFileRoute } from '@tanstack/react-router';
import { MigrationWizardPage } from '../../pages/MigrationWizardPage/MigrationWizardPage';

export const Route = createFileRoute('/_wizard/migration')({
  component: MigrationWizardPage,
});
