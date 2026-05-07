import './style.scss';
import clsx from 'clsx';
import type { PropsWithChildren } from 'react';
import { Icon } from '../../../defguard-ui/components/Icon';
import { IconButton } from '../../../defguard-ui/components/IconButton/IconButton';
import {
  policyOsVariantToIcon,
  policyOsVariantToText,
} from '../../../utils/policyPostures';
import type { PolicyOsVariant } from '../../SystemSelector/types';

interface Props extends PropsWithChildren {
  os: PolicyOsVariant;
  hideCard?: boolean;
  onDiscard?: () => void;
}

export const PolicyOsCard = ({ os, onDiscard, children, hideCard = false }: Props) => {
  return (
    <div
      className={clsx('policy-os-card', {
        card: !hideCard,
      })}
    >
      <div className="top">
        <div className="left">
          <Icon icon={policyOsVariantToIcon(os)} />
          <p>{policyOsVariantToText(os)}</p>
        </div>
        <div className="right">
          <IconButton
            icon="delete"
            onClick={() => {
              onDiscard?.();
            }}
          />
        </div>
      </div>
      <div className="content">{children}</div>
    </div>
  );
};
