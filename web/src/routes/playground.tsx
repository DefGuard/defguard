import { createFileRoute } from '@tanstack/react-router';
import { PlaygroundPage } from '../pages/PlaygroundPage/PlaygroundPage';

export const Route = createFileRoute('/playground')({
  component: PlaygroundPage,
});
