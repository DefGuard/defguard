import './style.scss';
import { Link } from '@tanstack/react-router';
import { useEffect, useMemo, useRef, useState } from 'react';
import Skeleton from 'react-loading-skeleton';
import { m } from '../../../../paraglide/messages';
import { Fold } from '../../../defguard-ui/components/Fold/Fold';
import { Icon } from '../../../defguard-ui/components/Icon/Icon';
import { IconButton } from '../../../defguard-ui/components/IconButton/IconButton';
import { ModalFoundation } from '../../../defguard-ui/components/ModalFoundation/ModalFoundation';
import { Direction } from '../../../defguard-ui/types';
import { useApp } from '../../../hooks/useApp';
import { useAllVideoTutorialsSections, useVideoTutorialsRouteKey } from '../../resolved';
import { getRouteLabel } from '../../route-label';
import type { VideoTutorial, VideoTutorialsSection } from '../../types';

const LOAD_TIMEOUT_MS = 8_000;

// ---------------------------------------------------------------------------
// Right panel: inline video player + metadata
// ---------------------------------------------------------------------------

interface VideoPlayerProps {
  video: VideoTutorial;
}

const VideoPlayer = ({ video }: VideoPlayerProps) => {
  const [loaded, setLoaded] = useState(false);
  const [errored, setErrored] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Reset state whenever the selected video changes.
  // biome-ignore lint/correctness/useExhaustiveDependencies: resetting on ID change is intentional; full object ref changes every render
  useEffect(() => {
    setLoaded(false);
    setErrored(false);

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

  const pageLabel = getRouteLabel(video.appRoute) ?? video.appRoute;

  return (
    <div className="tutorials-modal-player">
      <div className="tutorials-modal-iframe-wrapper">
        {errored ? (
          <div className="tutorials-modal-iframe-error">
            <Icon icon="tutorial-not-available" size={48} />
            <p>{m.cmp_video_tutorials_overlay_error()}</p>
            <a
              href={`https://www.youtube.com/watch?v=${video.youtubeVideoId}`}
              target="_blank"
              rel="noreferrer"
            >
              {m.cmp_video_tutorials_overlay_watch_on_youtube()}{' '}
              {`https://www.youtube.com/watch?v=${video.youtubeVideoId}`}
            </a>
          </div>
        ) : (
          <>
            {!loaded && (
              <div className="tutorials-modal-iframe-skeleton">
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

      <div className="tutorials-modal-video-info">
        <h3 className="tutorials-modal-video-title">{video.title}</h3>
        <p className="tutorials-modal-video-description">{video.description}</p>
        <div className="tutorials-modal-video-links">
          <Link
            to={video.appRoute}
            className="tutorials-modal-link tutorials-modal-link--internal"
            onClick={() => useApp.setState({ tutorialsModalOpen: false })}
          >
            <Icon icon="arrow-small" size={16} rotationDirection={Direction.RIGHT} />
            <span>{m.cmp_video_tutorials_modal_go_to({ page: pageLabel })}</span>
          </Link>
          <a
            href={video.docsUrl}
            target="_blank"
            rel="noreferrer"
            className="tutorials-modal-link tutorials-modal-link--external"
          >
            <Icon icon="open-in-new-window" size={16} />
            <span>{m.cmp_video_tutorials_modal_learn_more()}</span>
          </a>
        </div>
      </div>
    </div>
  );
};

// ---------------------------------------------------------------------------
// Left panel: search + section list
// ---------------------------------------------------------------------------

interface VideoListProps {
  sections: VideoTutorialsSection[];
  selectedVideo: VideoTutorial | null;
  onSelect: (video: VideoTutorial) => void;
}

const VideoList = ({ sections, selectedVideo, onSelect }: VideoListProps) => {
  const [search, setSearch] = useState('');
  const [openSectionIndex, setOpenSectionIndex] = useState<number | null>(0);

  // When sections change (modal opens/data reloads), reset accordion to first section.
  // biome-ignore lint/correctness/useExhaustiveDependencies: reset intentional on sections identity change
  useEffect(() => {
    setOpenSectionIndex(0);
  }, [sections]);

  const isSearching = search.trim().length > 0;

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return sections;
    return sections
      .map((section) => ({
        ...section,
        videos: section.videos.filter(
          (v) =>
            v.title.toLowerCase().includes(q) || section.name.toLowerCase().includes(q),
        ),
      }))
      .filter((s) => s.videos.length > 0);
  }, [sections, search]);

  const handleSectionToggle = (index: number, section: VideoTutorialsSection) => {
    setOpenSectionIndex((prev) => {
      if (prev === index) return null;
      // Opening a new section — select its first video
      if (section.videos.length > 0) {
        onSelect(section.videos[0]);
      }
      return index;
    });
  };

  return (
    <div className="tutorials-modal-list-panel">
      <div className="tutorials-modal-search-wrapper">
        <Icon icon="search" size={16} />
        <input
          type="search"
          className="tutorials-modal-search"
          placeholder={m.cmp_video_tutorials_modal_search_placeholder()}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </div>

      <div className="tutorials-modal-sections">
        {filtered.map((section, index) => {
          const isOpen = isSearching || openSectionIndex === index;
          return (
            <div key={section.name} className="tutorials-modal-section">
              <button
                type="button"
                className="tutorials-modal-section-header"
                onClick={() => handleSectionToggle(index, section)}
              >
                {section.name}
              </button>
              <Fold open={isOpen} contentClassName="tutorials-modal-section-videos-fold">
                <ul className="tutorials-modal-section-videos">
                  {section.videos.map((video) => {
                    const isSelected =
                      selectedVideo?.youtubeVideoId === video.youtubeVideoId;
                    return (
                      <li key={video.youtubeVideoId}>
                        <button
                          type="button"
                          className={`tutorials-modal-video-row${isSelected ? ' selected' : ''}`}
                          onClick={() => onSelect(video)}
                        >
                          <Icon icon={isSelected ? 'play-filled' : 'play'} size={16} />
                          <span>{video.title}</span>
                        </button>
                      </li>
                    );
                  })}
                </ul>
              </Fold>
            </div>
          );
        })}
      </div>
    </div>
  );
};

// ---------------------------------------------------------------------------
// Modal root
// ---------------------------------------------------------------------------

export const VideoTutorialsModal = () => {
  const isOpen = useApp((s) => s.tutorialsModalOpen);
  const sections = useAllVideoTutorialsSections();
  const routeKey = useVideoTutorialsRouteKey();

  const [selectedVideo, setSelectedVideo] = useState<VideoTutorial | null>(null);

  // Auto-select first video when modal opens or sections change.
  useEffect(() => {
    if (isOpen && sections.length > 0 && sections[0].videos.length > 0) {
      setSelectedVideo(sections[0].videos[0]);
    }
  }, [isOpen, sections]);

  // Close modal on route change.
  // biome-ignore lint/correctness/useExhaustiveDependencies: routeKey is the trigger, not used in body
  useEffect(() => {
    useApp.setState({ tutorialsModalOpen: false });
  }, [routeKey]);

  const handleClose = () => useApp.setState({ tutorialsModalOpen: false });

  return (
    <ModalFoundation
      isOpen={isOpen}
      contentClassName="tutorials-modal-container"
      afterClose={() => setSelectedVideo(null)}
    >
      <div className="tutorials-modal">
        <div className="tutorials-modal-header">
          <h2 className="tutorials-modal-title">{m.cmp_video_tutorials_modal_title()}</h2>
          <IconButton
            icon="close"
            className="tutorials-modal-close"
            onClick={handleClose}
          />
        </div>

        <div className="tutorials-modal-body">
          <VideoList
            sections={sections}
            selectedVideo={selectedVideo}
            onSelect={setSelectedVideo}
          />
          {selectedVideo && <VideoPlayer video={selectedVideo} />}
        </div>
      </div>
    </ModalFoundation>
  );
};
