import { useContext } from 'react';

import { ToastType } from '../components/layout/Toast/Toast';
import { ToasterContext } from '../contexts/ToasterContext';

export const useToaster = () => {
  const { eventObserver } = useContext(ToasterContext);

  const callToast = (type: ToastType, message: string, subMessage?: string) => {
    eventObserver.next({ type, message, subMessage });
  };

  const success = (message: string, subMessage?: string) =>
    callToast(ToastType.SUCCESS, message, subMessage);

  const info = (message: string, subMessage?: string) =>
    callToast(ToastType.INFO, message, subMessage);

  const warning = (message: string, subMessage?: string) =>
    callToast(ToastType.WARNING, message, subMessage);

  const error = (message: string, subMessage?: string) =>
    callToast(ToastType.ERROR, message, subMessage);
  return { success, info, warning, error };
};
