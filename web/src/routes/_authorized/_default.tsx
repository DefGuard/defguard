import { createFileRoute, Outlet } from '@tanstack/react-router';
import { Navigation } from '../../shared/components/Navigation/Navigation';
import { VideoSupportWidget } from '../../shared/video-tutorials/VideoSupportWidget';
import { VideoTutorialsModal } from '../../shared/video-tutorials/VideoTutorialsModal';

export const Route = createFileRoute('/_authorized/_default')({
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <>
      <Outlet />
      <Navigation />
      <VideoSupportWidget />
      <VideoTutorialsModal />
    </>
  );
}
