import { createFileRoute, Outlet } from '@tanstack/react-router';
import { Navigation } from '../../shared/components/Navigation/Navigation';
import { VideoSupportWidget } from '../../shared/video-support/widget';

export const Route = createFileRoute('/_authorized/_default')({
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <>
      <Outlet />
      <Navigation />
      <VideoSupportWidget />
    </>
  );
}
