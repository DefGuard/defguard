import { ToastType } from '../components/layout/ToastManager/Toast/Toast';
import { useToastsStore } from './store/useToastStore';

export const useToaster = () => {
  const addToast = useToastsStore((store) => store.addToast);

  const success = (message: string, subMessage?: string) =>
    addToast({
      type: ToastType.SUCCESS,
      message,
      subMessage,
    });

  const info = (message: string, subMessage?: string) =>
    addToast({
      type: ToastType.INFO,
      message,
      subMessage,
    });

  const warning = (message: string, subMessage?: string) =>
    addToast({
      type: ToastType.WARNING,
      message,
      subMessage,
    });

  const error = (message: string, subMessage?: string) =>
    addToast({
      type: ToastType.ERROR,
      message,
      subMessage,
    });

  return { success, info, warning, error };
};
