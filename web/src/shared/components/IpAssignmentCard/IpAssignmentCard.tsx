import './style.scss';
import type { PropsWithChildren } from 'react';
import { Fold } from '../../defguard-ui/components/Fold/Fold';
import { Icon, IconKind } from '../../defguard-ui/components/Icon';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { Direction, ThemeSpacing } from '../../defguard-ui/types';

interface Props extends PropsWithChildren {
  title: string;
  isOpen: boolean;
  onOpenChange: (value?: boolean) => void;
}

export const IpAssignmentCard = ({ isOpen, onOpenChange, title, children }: Props) => {
  return (
    <div className="location-assignment-card" data-open={isOpen}>
      <div
        className="main-track"
        onClick={() => {
          onOpenChange(!isOpen);
        }}
      >
        <Icon
          icon={IconKind.ArrowSmall}
          rotationDirection={isOpen ? Direction.DOWN : Direction.RIGHT}
        />
        <p className="title">{title}</p>
      </div>
      <Fold open={isOpen}>
        <SizedBox height={ThemeSpacing.Lg} />
        <div className="devices">{children}</div>
      </Fold>
    </div>
  );
};
