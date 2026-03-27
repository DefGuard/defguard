import './style.scss';
import { useEffect, useRef, useState } from 'react';
import { Modal } from '../defguard-ui/components/Modal/Modal';
import { useResolvedHelpTutorials } from './resolved';
import type { HelpTutorial } from './types';

// Inline SVG: simple video-camera icon for the launcher button
const VideoCameraIcon = () => (
  <svg
    width="16"
    height="16"
    viewBox="0 0 16 16"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    aria-hidden="true"
  >
    <rect x="1" y="3.5" width="9" height="9" rx="1.5" fill="currentColor" />
    <path d="M10 6.5L15 4v8l-5-2.5V6.5Z" fill="currentColor" />
  </svg>
);

/**
 * Returns the thumbnail URL for a tutorial.
 * Uses the explicit thumbnailUrl from the JSON if present, otherwise derives
 * one from the YouTube thumbnail endpoint using the video ID.
 */
function resolveThumbnailUrl(tutorial: HelpTutorial): string {
  return (
    tutorial.thumbnailUrl ??
    `https://img.youtube.com/vi/${tutorial.youtubeVideoId}/hqdefault.jpg`
  );
}

interface ThumbnailProps {
  url: string;
  title: string;
}

const Thumbnail = ({ url, title }: ThumbnailProps) => {
  const [loaded, setLoaded] = useState(false);
  const [errored, setErrored] = useState(false);

  if (errored) {
    return <div className="help-tutorial-thumbnail placeholder" aria-label={title} />;
  }

  return (
    <div className="help-tutorial-thumbnail">
      {!loaded && <div className="skeleton" />}
      <img
        src={url}
        alt={title}
        onLoad={() => setLoaded(true)}
        onError={() => setErrored(true)}
        style={{ display: loaded ? 'block' : 'none' }}
      />
    </div>
  );
};

interface TutorialCardProps {
  tutorial: HelpTutorial;
  onClick: () => void;
}

const TutorialCard = ({ tutorial, onClick }: TutorialCardProps) => (
  <button type="button" className="help-tutorial-card" onClick={onClick}>
    <Thumbnail url={resolveThumbnailUrl(tutorial)} title={tutorial.title} />
    <div className="help-tutorial-card-info">
      <span className="help-tutorial-card-title">{tutorial.title}</span>
    </div>
  </button>
);

export const HelpTutorialsWidget = () => {
  const tutorials = useResolvedHelpTutorials();
  const [panelOpen, setPanelOpen] = useState(false);
  const [selectedVideo, setSelectedVideo] = useState<HelpTutorial | null>(null);
  const prevTutorialsRef = useRef(tutorials);

  // Reset UI state when the tutorial set changes (i.e. route changed)
  useEffect(() => {
    if (prevTutorialsRef.current !== tutorials) {
      setPanelOpen(false);
      setSelectedVideo(null);
      prevTutorialsRef.current = tutorials;
    }
  }, [tutorials]);

  if (tutorials.length === 0) return null;

  const handleCardClick = (tutorial: HelpTutorial) => {
    setSelectedVideo(tutorial);
    setPanelOpen(false);
  };

  const handleModalClose = () => setSelectedVideo(null);

  return (
    <>
      <div className="help-tutorials-widget">
        {panelOpen && (
          <div className="help-tutorials-panel">
            <ul className="help-tutorials-list">
              {tutorials.map((t) => (
                <li key={t.youtubeVideoId}>
                  <TutorialCard tutorial={t} onClick={() => handleCardClick(t)} />
                </li>
              ))}
            </ul>
            <div className="help-tutorials-panel-footer">
              <button
                type="button"
                className="help-tutorials-close-btn"
                onClick={() => setPanelOpen(false)}
                aria-label="Close video support panel"
              >
                ×
              </button>
            </div>
          </div>
        )}
        <button
          type="button"
          className="help-tutorials-launcher"
          onClick={() => setPanelOpen((o) => !o)}
          aria-label="Video support"
          aria-expanded={panelOpen}
        >
          <VideoCameraIcon />
          <span>Video support</span>
        </button>
      </div>

      <Modal
        id="help-tutorials-modal"
        title={selectedVideo?.title ?? ''}
        isOpen={selectedVideo !== null}
        onClose={handleModalClose}
        afterClose={() => setSelectedVideo(null)}
      >
        {selectedVideo && (
          <div className="help-tutorials-video-wrapper">
            <iframe
              src={`https://www.youtube-nocookie.com/embed/${selectedVideo.youtubeVideoId}?autoplay=1`}
              title={selectedVideo.title}
              allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
              allowFullScreen
            />
          </div>
        )}
      </Modal>
    </>
  );
};
