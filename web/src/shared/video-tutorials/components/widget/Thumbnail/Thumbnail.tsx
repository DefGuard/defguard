import './style.scss';
import { useState } from 'react';
import Skeleton from 'react-loading-skeleton';
import { Icon } from '../../../../defguard-ui/components/Icon/Icon';
import { ThemeVariable } from '../../../../defguard-ui/types';

export interface ThumbnailProps {
  url: string;
  title: string;
}

export const Thumbnail = ({ url, title }: ThumbnailProps) => {
  const [loaded, setLoaded] = useState(false);
  const [errored, setErrored] = useState(false);

  if (errored) {
    return (
      <div className="video-tutorials-thumbnail placeholder" aria-label={title}>
        <div className="icon-badge">
          <Icon icon="tutorial" size={20} staticColor={ThemeVariable.FgDisabled} />
        </div>
      </div>
    );
  }

  return (
    <div className="video-tutorials-thumbnail">
      {!loaded && <Skeleton width={106} height={60} />}
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
