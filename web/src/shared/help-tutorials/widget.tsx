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

/** Resolved tutorial entry: thumbnailUrl is always present after validation. */
interface ValidatedTutorial {
  tutorial: HelpTutorial;
  thumbnailUrl: string;
}

/**
 * Returns the YouTube thumbnail URL to use for a given video ID.
 * The explicit thumbnailUrl from the JSON takes precedence; otherwise the
 * standard YouTube thumbnail endpoint is used.
 */
function resolveThumbnailUrl(tutorial: HelpTutorial): string {
  return (
    tutorial.thumbnailUrl ??
    `https://img.youtube.com/vi/${tutorial.youtubeVideoId}/hqdefault.jpg`
  );
}

/**
 * Probes thumbnail URLs for each tutorial in parallel.
 * Entries whose thumbnail returns a 404 (invalid/private/deleted video) are
 * silently discarded. Returns null while probing is in progress.
 */
function useValidatedTutorials(tutorials: HelpTutorial[]): ValidatedTutorial[] | null {
  const [validated, setValidated] = useState<ValidatedTutorial[] | null>(null);

  useEffect(() => {
    if (tutorials.length === 0) {
      setValidated([]);
      return;
    }

    setValidated(null); // mark as loading while probing

    let cancelled = false;

    const probes = tutorials.map(
      (tutorial) =>
        new Promise<ValidatedTutorial | null>((resolve) => {
          const url = resolveThumbnailUrl(tutorial);
          const img = new Image();
          img.onload = () => resolve({ tutorial, thumbnailUrl: url });
          img.onerror = () => resolve(null); // 404 or network error → discard
          img.src = url;
        }),
    );

    Promise.all(probes).then((results) => {
      if (cancelled) return;
      setValidated(results.filter((r): r is ValidatedTutorial => r !== null));
    });

    return () => {
      cancelled = true;
    };
  }, [tutorials]);

  return validated;
}

interface ThumbnailProps {
  url: string;
  title: string;
}

const Thumbnail = ({ url, title }: ThumbnailProps) => {
  const [loaded, setLoaded] = useState(false);

  return (
    <div className="help-tutorial-thumbnail">
      {!loaded && <div className="skeleton" />}
      <img
        src={url}
        alt={title}
        onLoad={() => setLoaded(true)}
        style={{ display: loaded ? 'block' : 'none' }}
      />
    </div>
  );
};

interface TutorialCardProps {
  entry: ValidatedTutorial;
  onClick: () => void;
}

const TutorialCard = ({ entry, onClick }: TutorialCardProps) => (
  <button type="button" className="help-tutorial-card" onClick={onClick}>
    <Thumbnail url={entry.thumbnailUrl} title={entry.tutorial.title} />
    <div className="help-tutorial-card-info">
      <span className="help-tutorial-card-title">{entry.tutorial.title}</span>
      {entry.tutorial.description && (
        <span className="help-tutorial-card-description">
          {entry.tutorial.description}
        </span>
      )}
    </div>
  </button>
);

export const HelpTutorialsWidget = () => {
  const tutorials = useResolvedHelpTutorials();
  const validated = useValidatedTutorials(tutorials);
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

  // Hide button while validation is in progress or no valid videos remain
  if (!validated || validated.length === 0) return null;

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
              {validated.map((entry) => (
                <li key={entry.tutorial.youtubeVideoId}>
                  <TutorialCard
                    entry={entry}
                    onClick={() => handleCardClick(entry.tutorial)}
                  />
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
