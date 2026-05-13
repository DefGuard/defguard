import { policyOsVariantToText } from '../../shared/utils/policyPostures';
import {
  PostureCheckOs,
  type PostureCheckOsValue,
  type PostureCheckOsVersionValue,
} from '../PostureChecksPage/types';

export const getOperatingSystemVersionOptionLabel = (
  operatingSystem: PostureCheckOsValue,
  value: PostureCheckOsVersionValue,
) => {
  switch (operatingSystem) {
    case PostureCheckOs.Windows:
    case PostureCheckOs.Macos:
      return `${policyOsVariantToText(operatingSystem)} ${value} or higher`;
    case PostureCheckOs.Linux:
      return `Kernel ${value} or higher`;
    case PostureCheckOs.Ios:
      return `iOS ${value} or higher`;
    case PostureCheckOs.Android:
      return `Android ${value} or higher`;
  }
};
