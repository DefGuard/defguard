import type { PropsWithChildren } from 'react';
import './style.scss';
import { isPresent } from '../../defguard-ui/utils/isPresent';

interface Props extends PropsWithChildren {
  imageSrc: string;
  title: string;
  subtitle: string;
}

export const ActionCard = ({ imageSrc, subtitle, title, children }: Props) => {
  return (
    <div className="action-card">
      <div className="inner-track">
        <div className="image-track">
          <img src={imageSrc} loading="lazy" />
        </div>
        <div className="content-track">
          <p className="title">{title}</p>
          <p className="subtitle">{subtitle}</p>
          {isPresent(children) && <div className="content">{children}</div>}
        </div>
      </div>
    </div>
  );
};
