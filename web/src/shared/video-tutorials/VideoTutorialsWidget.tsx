import './style.scss';
import { useEffect, useState } from 'react';
import { m } from '../../paraglide/messages';
import { Icon } from '../defguard-ui/components/Icon/Icon';
import { IconButton } from '../defguard-ui/components/IconButton/IconButton';
import { ThemeVariable } from '../defguard-ui/types';
import { VideoCard } from './components/VideoCard/VideoCard';
import { VideoOverlay } from './components/VideoOverlay/VideoOverlay';
import { useResolvedVideoTutorials, useVideoTutorialsRouteKey } from './resolved';
import type { VideoTutorial } from './types';

export const VideoTutorialsWidget = () => {
  const videos = useResolvedVideoTutorials();
  const routeKey = useVideoTutorialsRouteKey();
  const [panelOpen, setPanelOpen] = useState(false);
  const [overlayOpen, setOverlayOpen] = useState(false);
  const [selectedVideo, setSelectedVideo] = useState<VideoTutorial | null>(null);

  // Reset UI state when the route changes.
  // biome-ignore lint/correctness/useExhaustiveDependencies: routeKey is the trigger, not used in body
  useEffect(() => {
    setPanelOpen(false);
    setOverlayOpen(false);
    setSelectedVideo(null);
  }, [routeKey]);

  if (videos.length === 0) return null;

  const handleCardClick = (video: VideoTutorial) => {
    setSelectedVideo(video);
    setOverlayOpen(true);
    setPanelOpen(false);
  };

  return (
    <>
      <div className="video-tutorials-widget">
        {panelOpen && (
          <ul
            className="video-tutorials-list"
            aria-label={m.cmp_video_tutorials_list_label()}
          >
            {videos.map((v) => (
              <li key={v.youtubeVideoId}>
                <VideoCard video={v} onClick={() => handleCardClick(v)} />
              </li>
            ))}
          </ul>
        )}
        {panelOpen ? (
          <IconButton
            icon="close"
            className="video-tutorials-close-btn"
            onClick={() => setPanelOpen(false)}
          />
        ) : (
          <button
            type="button"
            className="video-tutorials-launcher"
            onClick={() => setPanelOpen(true)}
            aria-label={m.cmp_video_tutorials_launcher()}
          >
            <Icon icon="tutorial" size={18} staticColor={ThemeVariable.FgAction} />
            <span>{m.cmp_video_tutorials_launcher()}</span>
          </button>
        )}
      </div>

      <VideoOverlay
        video={selectedVideo}
        isOpen={overlayOpen}
        onClose={() => setOverlayOpen(false)}
        afterClose={() => setSelectedVideo(null)}
      />
    </>
  );
};
