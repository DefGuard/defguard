import './style.scss';
import { useEffect, useRef, useState } from 'react';
import Skeleton from 'react-loading-skeleton';
import * as m from '../../../../paraglide/messages';
import { Icon } from '../../../defguard-ui/components/Icon/Icon';
import { IconButton } from '../../../defguard-ui/components/IconButton/IconButton';
import { ModalFoundation } from '../../../defguard-ui/components/ModalFoundation/ModalFoundation';
import { ThemeVariable } from '../../../defguard-ui/types';
import type { VideoSupport } from '../../types';

const LOAD_TIMEOUT_MS = 8_000;

export interface VideoOverlayProps {
  video: VideoSupport | null;
  isOpen: boolean;
  onClose: () => void;
  afterClose: () => void;
}

export const VideoOverlay = ({
  video,
  isOpen,
  onClose,
  afterClose,
}: VideoOverlayProps) => {
  const [loaded, setLoaded] = useState(false);
  const [errored, setErrored] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    setLoaded(false);
    setErrored(false);

    if (!video) return;

    timeoutRef.current = setTimeout(() => {
      setErrored(true);
    }, LOAD_TIMEOUT_MS);

    return () => {
      if (timeoutRef.current !== null) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
    };
  }, [video]);

  const handleLoad = () => {
    if (timeoutRef.current !== null) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    setLoaded(true);
  };

  return (
    <ModalFoundation
      isOpen={isOpen}
      contentClassName="video-support-modal-container"
      afterClose={afterClose}
    >
      <IconButton icon="close" className="video-support-modal-close" onClick={onClose} />
      <div className="video-support-modal">
        {video &&
          (errored ? (
            <div className="video-support-overlay-error">
              <Icon icon="tutorial" size={40} staticColor={ThemeVariable.FgDisabled} />
              <p>{m.cmp_video_support_overlay_error()}</p>
            </div>
          ) : (
            <>
              {!loaded && (
                <div className="video-support-overlay-skeleton">
                  <Skeleton width="100%" height="100%" />
                </div>
              )}
              <iframe
                className={loaded ? 'loaded' : undefined}
                src={`https://www.youtube-noocookie.com/embed/${video.youtubeVideoId}?autoplay=1`}
                title={video.title}
                allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
                allowFullScreen
                sandbox="allow-scripts allow-same-origin allow-presentation allow-fullscreen"
                onLoad={handleLoad}
              />
            </>
          ))}
      </div>
    </ModalFoundation>
  );
};
