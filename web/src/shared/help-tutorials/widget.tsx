import './style.scss';
import { useEffect, useRef, useState } from 'react';
import { Icon } from '../defguard-ui/components/Icon/Icon';
import { IconButton } from '../defguard-ui/components/IconButton/IconButton';
import { ModalFoundation } from '../defguard-ui/components/ModalFoundation/ModalFoundation';
import { useResolvedHelpTutorials } from './resolved';
import type { HelpTutorial } from './types';

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
        className={loaded ? 'loaded' : undefined}
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
    <Thumbnail
      url={`https://img.youtube.com/vi/${tutorial.youtubeVideoId}/hqdefault.jpg`}
      title={tutorial.title}
    />
    <div className="help-tutorial-card-info">
      <span className="help-tutorial-card-title">{tutorial.title}</span>
    </div>
  </button>
);

interface VideoOverlayProps {
  tutorial: HelpTutorial;
  onClose: () => void;
}

const VideoOverlay = ({ tutorial, onClose }: VideoOverlayProps) => (
  <ModalFoundation
    isOpen
    onClose={onClose}
    afterClose={() => {}}
    contentClassName="help-tutorials-modal-container"
  >
    <IconButton icon="close" className="help-tutorials-modal-close" onClick={onClose} />
    <div className="help-tutorials-modal">
      <iframe
        src={`https://www.youtube-nocookie.com/embed/${tutorial.youtubeVideoId}?autoplay=1`}
        title={tutorial.title}
        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
        allowFullScreen
      />
    </div>
  </ModalFoundation>
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

  return (
    <>
      <div className="help-tutorials-widget">
        {panelOpen && (
          <ul className="help-tutorials-list" aria-label="Video tutorials">
            {tutorials.map((t) => (
              <li key={t.youtubeVideoId}>
                <TutorialCard tutorial={t} onClick={() => handleCardClick(t)} />
              </li>
            ))}
          </ul>
        )}
        {panelOpen ? (
          <IconButton
            icon="close"
            className="help-tutorials-close-btn"
            onClick={() => setPanelOpen(false)}
          />
        ) : (
          <button
            type="button"
            className="help-tutorials-launcher"
            onClick={() => setPanelOpen(true)}
            aria-label="Video support"
          >
            <Icon icon="tutorial" size={18} />
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
