import './style.scss';
import { useEffect, useState } from 'react';
import { m } from '../../paraglide/messages';
import { Icon } from '../defguard-ui/components/Icon/Icon';
import { IconButton } from '../defguard-ui/components/IconButton/IconButton';
import { ThemeVariable } from '../defguard-ui/types';
import { VideoCard } from './components/VideoCard/VideoCard';
import { VideoOverlay } from './components/VideoOverlay/VideoOverlay';
import { useResolvedVideoSupport, useVideoSupportRouteKey } from './resolved';
import type { VideoSupport } from './types';

export const VideoSupportWidget = () => {
  const videos = useResolvedVideoSupport();
  const routeKey = useVideoSupportRouteKey();
  const [panelOpen, setPanelOpen] = useState(false);
  const [overlayOpen, setOverlayOpen] = useState(false);
  const [selectedVideo, setSelectedVideo] = useState<VideoSupport | null>(null);

  // Reset UI state when the route changes.
  // biome-ignore lint/correctness/useExhaustiveDependencies: routeKey is the trigger, not used in body
  useEffect(() => {
    setPanelOpen(false);
    setOverlayOpen(false);
    setSelectedVideo(null);
  }, [routeKey]);

  if (videos.length === 0) return null;

  const handleCardClick = (video: VideoSupport) => {
    setSelectedVideo(video);
    setOverlayOpen(true);
    setPanelOpen(false);
  };

  return (
    <>
      <div className="video-support-widget">
        {panelOpen && (
          <ul
            className="video-support-list"
            aria-label={m.cmp_video_support_list_label()}
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
            className="video-support-close-btn"
            onClick={() => setPanelOpen(false)}
          />
        ) : (
          <button
            type="button"
            className="video-support-launcher"
            onClick={() => setPanelOpen(true)}
            aria-label={m.cmp_video_support_launcher()}
          >
            <Icon icon="tutorial" size={18} staticColor={ThemeVariable.FgAction} />
            <span>{m.cmp_video_support_launcher()}</span>
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
