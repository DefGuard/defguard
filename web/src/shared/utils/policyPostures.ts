import type { PolicyOsVariant } from '../components/SystemSelector/types';
import type { IconKindValue } from '../defguard-ui/components/Icon';

export const policyOsVariantToText = (variant: PolicyOsVariant): string => {
  switch (variant) {
    case 'windows':
      return 'Windows';
    case 'android':
      return 'Android';
    case 'ios':
      return 'iOS';
    case 'linux':
      return 'Linux';
    case 'macos':
      return 'macOS';
  }
};

export const policyOsVariantToIcon = (variant: PolicyOsVariant): IconKindValue => {
  switch (variant) {
    case 'android':
      return 'android';
    case 'ios':
      return 'app-store';
    case 'linux':
      return 'linux';
    case 'macos':
      return 'apple';
    case 'windows':
      return 'windows';
  }
};
