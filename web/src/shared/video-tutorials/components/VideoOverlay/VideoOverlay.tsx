import './style.scss';
import { IconButton } from '../../../defguard-ui/components/IconButton/IconButton';
import { ModalFoundation } from '../../../defguard-ui/components/ModalFoundation/ModalFoundation';
import type { VideoTutorial } from '../../types';
import { VideoPlayer } from '../VideoPlayer/VideoPlayer';

export interface VideoOverlayProps {
  video: VideoTutorial | null;
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
  return (
    <ModalFoundation
      isOpen={isOpen}
      contentClassName="video-tutorials-modal-container"
      afterClose={afterClose}
    >
      {video && (
        <>
          <IconButton
            icon="close"
            className="video-tutorials-modal-close"
            onClick={onClose}
          />
          <div className="video-tutorials-modal">
            <VideoPlayer video={video} errorVariant="overlay" />
          </div>
        </>
      )}
    </ModalFoundation>
  );
};
