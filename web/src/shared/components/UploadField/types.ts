import type { ReactNode } from 'react';
import type { IconKindValue } from '../../defguard-ui/components/Icon/icon-types';

export interface UploadFieldProps {
  value?: File | null;
  stored?: boolean;
  label?: string;
  icon?: IconKindValue;
  rowEnd?: ReactNode;
  className?: string;
  id?: string;
  error?: string;
  loading?: boolean;
  disabled?: boolean;
  acceptedExtensions?: string[];
  testId?: string;
  title?: string;
  onChange?: (value: File | null) => void;
}
