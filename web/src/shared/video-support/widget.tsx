import './style.scss';
import { useEffect, useState } from 'react';
import Skeleton from 'react-loading-skeleton';
import { m } from '../../paraglide/messages';
import { Icon } from '../defguard-ui/components/Icon/Icon';
import { IconButton } from '../defguard-ui/components/IconButton/IconButton';
import { ModalFoundation } from '../defguard-ui/components/ModalFoundation/ModalFoundation';
import { useResolvedVideoSupport, useVideoSupportRouteKey } from './resolved';
import type { VideoSupport } from './types';

interface ThumbnailProps {
  url: string;
  title: string;
}

const Thumbnail = ({ url, title }: ThumbnailProps) => {
  const [loaded, setLoaded] = useState(false);
  const [errored, setErrored] = useState(false);

  if (errored) {
    return <div className="video-support-thumbnail placeholder" aria-label={title} />;
  }

  return (
    <div className="video-support-thumbnail">
      {!loaded && <Skeleton width={106} height={60} />}
      <img
        src={url}
        alt={title}
        onLoad={() => setLoaded(true)}
        onError={() => setErrored(true)}
        className={loaded ? 'loaded' : undefined}
      />
    </div>
  );
};

interface VideoCardProps {
  video: VideoSupport;
  onClick: () => void;
}

const VideoCard = ({ video, onClick }: VideoCardProps) => (
  <button type="button" className="video-support-card" onClick={onClick}>
    <Thumbnail
      url={`https://img.youtube.com/vi/${video.youtubeVideoId}/hqdefault.jpg`}
      title={video.title}
    />
    <div className="video-support-card-info">
      <span className="video-support-card-title">{video.title}</span>
    </div>
  </button>
);

interface VideoOverlayProps {
  video: VideoSupport | null;
  isOpen: boolean;
  onClose: () => void;
  afterClose: () => void;
}

const VideoOverlay = ({ video, isOpen, onClose, afterClose }: VideoOverlayProps) => (
  <ModalFoundation
    isOpen={isOpen}
    contentClassName="video-support-modal-container"
    afterClose={afterClose}
  >
    <IconButton icon="close" className="video-support-modal-close" onClick={onClose} />
    <div className="video-support-modal">
      {video && (
        <iframe
          src={`https://www.youtube-nocookie.com/embed/${video.youtubeVideoId}?autoplay=1`}
          title={video.title}
          allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
          allowFullScreen
          sandbox="allow-scripts allow-same-origin allow-presentation allow-fullscreen"
        />
      )}
    </div>
  </ModalFoundation>
);

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
            <Icon icon="tutorial" size={18} staticColor="var(--fg-action)" />
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
