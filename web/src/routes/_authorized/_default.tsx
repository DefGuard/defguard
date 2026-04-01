import { createFileRoute, Outlet } from '@tanstack/react-router';
import { Navigation } from '../../shared/components/Navigation/Navigation';
import { VideoTutorialsModal } from '../../shared/video-tutorials/VideoTutorialsModal';
import { VideoTutorialsWidget } from '../../shared/video-tutorials/VideoTutorialsWidget';

export const Route = createFileRoute('/_authorized/_default')({
  component: RouteComponent,
});

function RouteComponent() {
  return (
    <>
      <Outlet />
      <Navigation />
      <VideoTutorialsWidget />
      <VideoTutorialsModal />
    </>
  );
}
