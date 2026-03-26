import { createFileRoute } from '@tanstack/react-router';
import { EnrollmentPage } from '../../../pages/EnrollmentPage/EnrollmentPage';

export const Route = createFileRoute('/_authorized/_default/enrollment')({
  component: EnrollmentPage,
});
