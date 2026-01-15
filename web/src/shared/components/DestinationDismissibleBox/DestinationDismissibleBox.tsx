import './style.scss';
import type { MouseEventHandler } from 'react';
import { IconButton } from '../../defguard-ui/components/IconButton/IconButton';
import { DestinationLabel } from '../DestinationLabel/DestinationLabel';
import type { DestinationLabelProps } from '../DestinationLabel/types';

interface Props extends DestinationLabelProps {
  onClick: MouseEventHandler<HTMLDivElement>;
}

export const DestinationDismissibleBox = ({ onClick, ...rest }: Props) => {
  return (
    <div className="destination-dismissible-box">
      <div className="track">
        <DestinationLabel {...rest} />
        <IconButton icon="close" onClick={onClick} />
      </div>
    </div>
  );
};
