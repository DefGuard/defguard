import './style.scss';
import type { VideoSupport } from '../../types';
import { Thumbnail } from '../Thumbnail/Thumbnail';

export interface VideoCardProps {
  video: VideoSupport;
  onClick: () => void;
}

export const VideoCard = ({ video, onClick }: VideoCardProps) => (
  <button type="button" className="video-support-card" onClick={onClick}>
    <Thumbnail
      url={`https://img.youtube.com/vi/${video.youtubeVideoId}/hqdefault.jpg`}
      title={video.title}
    />
    <div className="video-support-card-info">
      <span className="video-support-card-title">{video.title}</span>
    </div>
  </button>
);
