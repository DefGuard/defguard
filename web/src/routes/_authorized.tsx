import { createFileRoute, Outlet } from '@tanstack/react-router';
import { Navigation } from '../shared/components/Navigation/Navigation';

export const Route = createFileRoute('/_authorized')({
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <>
      <Outlet />
      <Navigation />
    </>
  );
}
