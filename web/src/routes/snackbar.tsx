import { createFileRoute } from '@tanstack/react-router';
import { TestSnackbarPage } from '../pages/TestSnackbarPage/TestSnackbarPage';

export const Route = createFileRoute('/snackbar')({
  component: TestSnackbarPage,
});
