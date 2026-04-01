import './style.scss';
import { Link } from '@tanstack/react-router';
import { useEffect, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import { Icon } from '../../../defguard-ui/components/Icon/Icon';
import { IconButton } from '../../../defguard-ui/components/IconButton/IconButton';
import { ModalFoundation } from '../../../defguard-ui/components/ModalFoundation/ModalFoundation';
import { Direction } from '../../../defguard-ui/types';
import { useApp } from '../../../hooks/useApp';
import { useAllVideoTutorialsSections, useVideoTutorialsRouteKey } from '../../resolved';
import { getRouteLabel } from '../../route-label';
import type { VideoTutorial } from '../../types';
import { VideoList } from '../VideoList/VideoList';
import { VideoPlayer } from '../VideoPlayer/VideoPlayer';

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
      contentClassName="tutorials-modal"
      afterClose={() => setSelectedVideo(null)}
    >
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
        {selectedVideo && (
          <div className="tutorials-modal-player">
            <div className="tutorials-modal-iframe-wrapper">
              <VideoPlayer video={selectedVideo} errorVariant="modal" resetOnChange />
            </div>

            <div className="tutorials-modal-video-info">
              <div className="tutorials-modal-video-text">
                <h3 className="tutorials-modal-video-title">{selectedVideo.title}</h3>
                <p className="tutorials-modal-video-description">
                  {selectedVideo.description}
                </p>
              </div>
              <div className="tutorials-modal-video-links">
                <Link
                  to={selectedVideo.appRoute}
                  className="tutorials-modal-link tutorials-modal-link--internal"
                  onClick={() => useApp.setState({ tutorialsModalOpen: false })}
                >
                  <Icon
                    icon="arrow-small"
                    size={16}
                    rotationDirection={Direction.RIGHT}
                  />
                  <span>
                    {m.cmp_video_tutorials_modal_go_to({
                      page:
                        getRouteLabel(selectedVideo.appRoute) ?? selectedVideo.appRoute,
                    })}
                  </span>
                </Link>
                <a
                  href={selectedVideo.docsUrl}
                  target="_blank"
                  rel="noreferrer"
                  className="tutorials-modal-link tutorials-modal-link--external"
                >
                  <Icon
                    icon="arrow-small"
                    size={16}
                    rotationDirection={Direction.RIGHT}
                  />
                  <span>{m.cmp_video_tutorials_modal_learn_more()}</span>
                  <Icon icon="open-in-new-window" size={16} />
                </a>
              </div>
            </div>
          </div>
        )}
      </div>
    </ModalFoundation>
  );
};
