import './style.scss';
import { useEffect, useRef, useState } from 'react';
import Skeleton from 'react-loading-skeleton';
import * as m from '../../../../paraglide/messages';
import { Icon } from '../../../defguard-ui/components/Icon/Icon';
import { IconButton } from '../../../defguard-ui/components/IconButton/IconButton';
import { ModalFoundation } from '../../../defguard-ui/components/ModalFoundation/ModalFoundation';
import type { VideoTutorial } from '../../types';

const LOAD_TIMEOUT_MS = 8_000;

export interface VideoOverlayProps {
  video: VideoTutorial | null;
  isOpen: boolean;
  onClose: () => void;
  afterClose: () => void;
}

interface VideoOverlayContentProps {
  video: VideoTutorial;
  onClose: () => void;
}

const VideoOverlayContent = ({ video, onClose }: VideoOverlayContentProps) => {
  const [loaded, setLoaded] = useState(false);
  const [errored, setErrored] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    timeoutRef.current = setTimeout(() => {
      setErrored(true);
    }, LOAD_TIMEOUT_MS);

    return () => {
      if (timeoutRef.current !== null) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
    };
  }, []);

  const handleLoad = () => {
    if (timeoutRef.current !== null) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    setLoaded(true);
  };

  return (
    <>
      <IconButton
        icon="close"
        className="video-tutorials-modal-close"
        onClick={onClose}
      />
      <div className="video-tutorials-modal">
        {errored ? (
          <div className="video-tutorials-overlay-error">
            <div className="video-tutorials-overlay-error-icon-group">
              <div className="video-tutorials-overlay-error-badge">
                <Icon icon="tutorial-not-available" size={48} />
              </div>
              <p className="video-tutorials-overlay-error-title">
                {m.cmp_video_tutorials_overlay_error()}
              </p>
            </div>
            <div className="video-tutorials-overlay-error-link-group">
              <p className="video-tutorials-overlay-error-label">
                {m.cmp_video_tutorials_overlay_watch_on_youtube()}
              </p>
              <a
                className="video-tutorials-overlay-error-url"
                href={`https://www.youtube.com/watch?v=${video.youtubeVideoId}`}
                target="_blank"
                rel="noreferrer"
              >
                {`https://www.youtube.com/watch?v=${video.youtubeVideoId}`}
              </a>
            </div>
          </div>
        ) : (
          <>
            {!loaded && (
              <div className="video-tutorials-overlay-skeleton">
                <Skeleton width="100%" height="100%" />
              </div>
            )}
            <iframe
              className={loaded ? 'loaded' : undefined}
              src={`https://www.youtube-nocookie.com/embed/${video.youtubeVideoId}`}
              title={video.title}
              allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
              allowFullScreen
              sandbox="allow-scripts allow-same-origin allow-presentation allow-fullscreen"
              onLoad={handleLoad}
            />
          </>
        )}
      </div>
    </>
  );
};

export const VideoOverlay = ({
  video,
  isOpen,
  onClose,
  afterClose,
}: VideoOverlayProps) => {
  return (
    <ModalFoundation
      isOpen={isOpen}
      contentClassName="video-tutorials-modal-container"
      afterClose={afterClose}
    >
      {video && <VideoOverlayContent video={video} onClose={onClose} />}
    </ModalFoundation>
  );
};
