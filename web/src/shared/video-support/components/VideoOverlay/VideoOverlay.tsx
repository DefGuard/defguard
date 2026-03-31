import './style.scss';
import { IconButton } from '../../../defguard-ui/components/IconButton/IconButton';
import { ModalFoundation } from '../../../defguard-ui/components/ModalFoundation/ModalFoundation';
import type { VideoSupport } from '../../types';

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
}: VideoOverlayProps) => (
  <ModalFoundation
    isOpen={isOpen}
    contentClassName="video-support-modal-container"
    afterClose={afterClose}
  >
    <IconButton icon="close" className="video-support-modal-close" onClick={onClose} />
    <div className="video-support-modal">
      {video && (
        <iframe
          src={`https://www.youtube-nocookie.com/embed/${video.youtubeVideoId}?autoplay=1`}
          title={video.title}
          allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
          allowFullScreen
          sandbox="allow-scripts allow-same-origin allow-presentation allow-fullscreen"
        />
      )}
    </div>
  </ModalFoundation>
);
