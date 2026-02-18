import { createFileRoute, redirect } from '@tanstack/react-router';
import { PlaygroundPage } from '../../pages/PlaygroundPage/PlaygroundPage';

export const Route = createFileRoute('/_authorized/playground')({
  beforeLoad: () => {
    if (import.meta.env.PROD) {
      throw redirect({ to: '/404', replace: true });
    }
  },
  component: PlaygroundPage,
});
