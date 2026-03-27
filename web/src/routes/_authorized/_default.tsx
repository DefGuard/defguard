import { createFileRoute, Outlet } from '@tanstack/react-router';
import { Navigation } from '../../shared/components/Navigation/Navigation';
import { HelpTutorialsWidget } from '../../shared/help-tutorials/widget';

export const Route = createFileRoute('/_authorized/_default')({
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <>
      <Outlet />
      <Navigation />
      <HelpTutorialsWidget />
    </>
  );
}
