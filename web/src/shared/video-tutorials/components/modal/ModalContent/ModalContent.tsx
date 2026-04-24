import { Link } from '@tanstack/react-router';
import { m } from '../../../../../paraglide/messages';
import { Icon } from '../../../../defguard-ui/components/Icon/Icon';
import { IconButton } from '../../../../defguard-ui/components/IconButton/IconButton';
import { Direction } from '../../../../defguard-ui/types';
import { isPresent } from '../../../../defguard-ui/utils/isPresent';
import { getNavRoot } from '../../../route-key';
import { getRouteLabel } from '../../../route-label';
import { useVideoTutorialsModal } from '../../../store';
import type { VideoTutorial, VideoTutorialsSection } from '../../../types';
import { VideoPlayer } from '../../VideoPlayer/VideoPlayer';
import { VideoList } from '../VideoList/VideoList';

type ModalContentProps = {
  selectedVideo: VideoTutorial;
  sections: VideoTutorialsSection[];
  onSelect: (video: VideoTutorial) => void;
  handleClose: () => void;
};

export const ModalContent = ({
  selectedVideo,
  sections,
  onSelect,
  handleClose,
}: ModalContentProps) => {
  const navRoot = getNavRoot(selectedVideo.appRoute);
  const label = getRouteLabel(navRoot) ?? navRoot;
  return (
    <>
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
          onSelect={onSelect}
        />
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
                to={navRoot}
                className="tutorials-modal-link tutorials-modal-link--internal"
                onClick={() => useVideoTutorialsModal.setState({ isOpen: false })}
              >
                <Icon icon="arrow-small" size={16} rotationDirection={Direction.RIGHT} />
                <span>
                  {m.cmp_video_tutorials_modal_go_to({
                    page: label,
                  })}
                </span>
              </Link>
              {isPresent(selectedVideo.docsUrl) && (
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
              )}
            </div>
          </div>
        </div>
      </div>
    </>
  );
};
