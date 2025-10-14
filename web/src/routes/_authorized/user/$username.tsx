import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/_authorized/user/$username')({
  component: RouteComponent,
});

function RouteComponent() {
  return <div>Hello "/_authorized/user/$username"!</div>;
}
