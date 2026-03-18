import { createFileRoute } from '@tanstack/react-router';
import { Error404Page } from '../pages/Error404Page/Error404Page';

export const Route = createFileRoute('/404')({
  component: Error404Page,
});
