import './style.scss';
import { useEffect, useRef, useState } from 'react';
import Skeleton from 'react-loading-skeleton';
import { m } from '../../../../paraglide/messages';
import { Icon } from '../../../defguard-ui/components/Icon/Icon';
import type { PlayableVideo } from '../../types';

const LOAD_TIMEOUT_MS = 8_000;

export type VideoPlayerErrorVariant = 'overlay' | 'modal';

export interface VideoPlayerProps {
  video: PlayableVideo;
  /**
   * Controls which error UI is rendered when the iframe fails to load.
   *
   * - `"overlay"` — richer layout with icon-group / link-group structure (used by VideoOverlay).
   * - `"modal"` — compact inline layout (used by VideoTutorialsModal).
   */
  errorVariant: VideoPlayerErrorVariant;
  /**
   * When `true`, the loaded/errored state is reset whenever `video.youtubeVideoId` changes.
   * Set to `true` in the modal (multiple videos), leave `false` in the overlay (single video).
   * @default false
   */
  resetOnChange?: boolean;
}

export const VideoPlayer = ({
  video,
  errorVariant,
  resetOnChange = false,
}: VideoPlayerProps) => {
  const [loaded, setLoaded] = useState(false);
  const [errored, setErrored] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // biome-ignore lint/correctness/useExhaustiveDependencies: resetOnChange is stable; video.youtubeVideoId is the intentional trigger
  useEffect(() => {
    if (resetOnChange) {
      setLoaded(false);
      setErrored(false);
    }

    timeoutRef.current = setTimeout(() => {
      setErrored(true);
    }, LOAD_TIMEOUT_MS);

    return () => {
      if (timeoutRef.current !== null) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
    };
  }, [video.youtubeVideoId]);

  const handleLoad = () => {
    if (timeoutRef.current !== null) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    setLoaded(true);
  };

  const youtubeUrl = `https://www.youtube.com/watch?v=${video.youtubeVideoId}`;

  if (errored) {
    if (errorVariant === 'overlay') {
      return (
        <div className="video-player-error video-player-error-overlay">
          <div className="video-player-error-icon-group">
            <div className="video-player-error-badge">
              <Icon icon="tutorial-not-available" size={48} />
            </div>
            <p className="video-player-error-title">
              {m.cmp_video_tutorials_overlay_error()}
            </p>
          </div>
          <div className="video-player-error-link-group">
            <p className="video-player-error-label">
              {m.cmp_video_tutorials_overlay_watch_on_youtube()}
            </p>
            <a
              className="video-player-error-url"
              href={youtubeUrl}
              target="_blank"
              rel="noreferrer"
            >
              {youtubeUrl}
            </a>
          </div>
        </div>
      );
    }

    return (
      <div className="video-player-error video-player-error-modal">
        <Icon icon="tutorial-not-available" size={48} />
        <p>{m.cmp_video_tutorials_overlay_error()}</p>
        <a href={youtubeUrl} target="_blank" rel="noreferrer">
          {m.cmp_video_tutorials_overlay_watch_on_youtube()} {youtubeUrl}
        </a>
      </div>
    );
  }

  return (
    <>
      {!loaded && (
        <div className="video-player-skeleton">
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
  );
};
