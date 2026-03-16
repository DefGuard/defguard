import './style.scss';
import type { PropsWithChildren, ReactNode } from 'react';
import { ModalFoundation } from '../../../defguard-ui/components/ModalFoundation/ModalFoundation';
import type { ModalBase } from '../../../defguard-ui/components/ModalFoundation/types';
import { isPresent } from '../../../defguard-ui/utils/isPresent';
import gridImage from './grid.png?url';

type Props = Omit<ModalBase, 'title'> & {
  image?: ReactNode;
  lines?: boolean;
} & PropsWithChildren;

export const LicenseModal = ({
  image,
  children,
  lines = false,
  ...foundationProps
}: Props) => {
  return (
    <ModalFoundation contentClassName="license-modal" {...foundationProps}>
      <div className="tracks">
        <div className="image-track">
          {lines && (
            <div
              className="lines"
              style={{
                backgroundImage: `url(${gridImage})`,
              }}
            ></div>
          )}
          {isPresent(image) && image}
        </div>
        <div className="content-track">{children}</div>
      </div>
    </ModalFoundation>
  );
};
