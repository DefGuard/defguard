import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/auth/mfa/recovery')({
  component: RouteComponent,
});

function RouteComponent() {
  return <div>Hello "/auth/mfa/recovery"!</div>;
}
