import './style.scss';
import { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { IconClose } from '../defguard-ui/components/Icon/icons/IconClose';
import { IconTutorial } from '../defguard-ui/components/Icon/icons/IconTutorial';
import { useResolvedHelpTutorials } from './resolved';
import type { HelpTutorial } from './types';

function resolveThumbnailUrl(tutorial: HelpTutorial): string {
  return `https://img.youtube.com/vi/${tutorial.youtubeVideoId}/hqdefault.jpg`;
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

interface VideoOverlayProps {
  tutorial: HelpTutorial;
  onClose: () => void;
}

const VideoOverlay = ({ tutorial, onClose }: VideoOverlayProps) => {
  // Close on Escape key
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [onClose]);

  const modalsRoot = document.getElementById('modals-root');
  if (!modalsRoot) return null;

  return createPortal(
    <div
      className="help-tutorials-overlay"
      onClick={onClose}
      aria-modal="true"
      role="dialog"
      aria-label={tutorial.title}
    >
      <div
        className="help-tutorials-modal-container"
        onClick={(e) => e.stopPropagation()}
      >
        <button
          type="button"
          className="help-tutorials-modal-close"
          onClick={onClose}
          aria-label="Close video"
        >
          <IconClose aria-hidden="true" />
        </button>
        <div className="help-tutorials-modal">
          <iframe
            src={`https://www.youtube-nocookie.com/embed/${tutorial.youtubeVideoId}?autoplay=1`}
            title={tutorial.title}
            allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
            allowFullScreen
          />
        </div>
      </div>
    </div>,
    modalsRoot,
  );
};

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

  return (
    <>
      <div className="help-tutorials-widget">
        {panelOpen && (
          <ul className="help-tutorials-list">
            {tutorials.map((t) => (
              <li key={t.youtubeVideoId}>
                <TutorialCard tutorial={t} onClick={() => handleCardClick(t)} />
              </li>
            ))}
          </ul>
        )}
        {panelOpen ? (
          <button
            type="button"
            className="help-tutorials-close-btn"
            onClick={() => setPanelOpen(false)}
            aria-label="Close video support panel"
          >
            ×
          </button>
        ) : (
          <button
            type="button"
            className="help-tutorials-launcher"
            onClick={() => setPanelOpen(true)}
            aria-label="Video support"
          >
            <IconTutorial aria-hidden="true" />
            <span>Video support</span>
          </button>
        )}
      </div>

      {selectedVideo && (
        <VideoOverlay tutorial={selectedVideo} onClose={() => setSelectedVideo(null)} />
      )}
    </>
  );
};
