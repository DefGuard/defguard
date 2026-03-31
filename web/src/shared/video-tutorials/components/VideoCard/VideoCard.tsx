import './style.scss';
import type { VideoTutorial } from '../../types';
import { Thumbnail } from '../Thumbnail/Thumbnail';

export interface VideoCardProps {
  video: VideoTutorial;
  onClick: () => void;
}

export const VideoCard = ({ video, onClick }: VideoCardProps) => (
  <button type="button" className="video-tutorials-card" onClick={onClick}>
    <Thumbnail
      url={`https://img.youtube.com/vi/${video.youtubeVideoId}/hqdefault.jpg`}
      title={video.title}
    />
    <div className="video-tutorials-card-info">
      <span className="video-tutorials-card-title">{video.title}</span>
    </div>
  </button>
);
