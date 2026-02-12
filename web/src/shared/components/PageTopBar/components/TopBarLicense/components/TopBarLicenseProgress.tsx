import clsx from 'clsx';
import { Icon } from '../../../../../defguard-ui/components/Icon';
import { ProgressionBar } from '../../../../../defguard-ui/components/ProgressionBar/ProgressionBar';
import { isPresent } from '../../../../../defguard-ui/utils/isPresent';
import type { TopBarLicenseProgressProps } from '../types';

export const TopBarLicenseProgress = ({
  icon,
  value,
  maxValue,
  label,
}: TopBarLicenseProgressProps) => {
  if (!isPresent(label))
    return (
      <div className="top-bar-license-progress-compact">
        <Icon icon={icon} size={16} />
        <span>{`${value}/${maxValue}`}</span>
        <ProgressionBar value={value} maxValue={maxValue} />
      </div>
    );

  return (
    <div className="top-bar-license-progress">
      <div className="top">
        <Icon icon={icon} size={16} />
        <span>{label}</span>
        <span
          className={clsx('right', {
            critical: value === maxValue,
          })}
        >{`${value}/${maxValue}`}</span>
      </div>
      <ProgressionBar value={value} maxValue={maxValue} />
    </div>
  );
};
