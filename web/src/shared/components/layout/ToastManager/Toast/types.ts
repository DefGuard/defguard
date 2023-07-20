import { ToastOptions } from '../../../../hooks/store/useToastStore';

export enum ToastType {
  INFO = 'info',
  WARNING = 'warning',
  SUCCESS = 'success',
  ERROR = 'error',
}

export interface ToastProps {
  data: ToastOptions;
}
