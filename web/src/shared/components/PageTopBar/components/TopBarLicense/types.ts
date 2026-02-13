import type { IconKindValue } from '../../../../defguard-ui/components/Icon';

export type TopBarLicenseProgressProps = {
  icon: IconKindValue;
  label?: string;
  value: number;
  maxValue: number;
};
