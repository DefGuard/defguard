import clsx from 'clsx';
import { Badge } from '../../defguard-ui/components/Badge/Badge';
import { Icon } from '../../defguard-ui/components/Icon';
import { ThemeVariable } from '../../defguard-ui/types';
import { isPresent } from '../../defguard-ui/utils/isPresent';
import './style.scss';
import type { EditHeaderProps } from './types';

export const EditHeader = ({ icon, badgeProps, title, subtitle }: EditHeaderProps) => {
  return (
    <div className="edit-header">
      <div
        className={clsx('inner-track', {
          split: isPresent(icon),
        })}
      >
        {isPresent(icon) && (
          <div className="icon-track">
            <div className="icon-wrap">
              <div className="bg"></div>
              <Icon icon={icon} size={20} staticColor={ThemeVariable.FgAction} />
            </div>
          </div>
        )}
        <div className="content-track">
          <div className="top">
            <h4>{title}</h4>
            {isPresent(badgeProps) && <Badge {...badgeProps} />}
          </div>
          {isPresent(subtitle) && <p>{subtitle}</p>}
        </div>
      </div>
    </div>
  );
};
