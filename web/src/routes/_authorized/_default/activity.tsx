import { createFileRoute } from '@tanstack/react-router';
import { ActivityLogPage } from '../../../pages/ActivityLogPage/ActivityLogPage';

export const Route = createFileRoute('/_authorized/_default/activity')({
  component: ActivityLogPage,
});
