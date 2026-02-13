import './style.scss';
import type { PropsWithChildren } from 'react';
import { ModalFoundation } from '../../../defguard-ui/components/ModalFoundation/ModalFoundation';
import type { ModalBase } from '../../../defguard-ui/components/ModalFoundation/types';
import gridImage from './grid.png?url';

type Props = Omit<ModalBase, 'title'> & {
  image?: string;
} & PropsWithChildren;

export const LicenseModal = ({ image, children, ...foundationProps }: Props) => {
  return (
    <ModalFoundation contentClassName="license-modal" {...foundationProps}>
      <div className="tracks">
        <div className="image-track">
          <div
            className="lines"
            style={{
              backgroundImage: `url(${gridImage})`,
            }}
          ></div>
          <div
            className="image"
            style={{
              backgroundImage: `url(${image})`,
            }}
          ></div>
        </div>
        <div className="content-track">{children}</div>
      </div>
    </ModalFoundation>
  );
};
