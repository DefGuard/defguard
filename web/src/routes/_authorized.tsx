import { createFileRoute, Outlet } from '@tanstack/react-router';

export const Route = createFileRoute('/_authorized')({
  component: RouteComponent,
});

function RouteComponent() {
  return <Outlet />;
}
