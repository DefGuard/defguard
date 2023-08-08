import { useCallback } from 'react';
import { useToaster } from './useToaster';
import { useI18nContext } from '../../i18n/i18n-react';

export const useClipboard = () => {
  const { LL } = useI18nContext();

  const toaster = useToaster();

  const writeToClipboard = useCallback((content: string, customMessage?: string) => {
    if (window.isSecureContext) {
      navigator.clipboard
        .writeText(content)
        .then(() => {
          if (customMessage && customMessage.length) {
            toaster.success(customMessage);
          } else {
            toaster.success(LL.messages.clipboard.success());
          }
        })
        .catch((e) => {
          toaster.error(LL.messages.clipboard.error());
          console.error(e);
        });
    } else {
      toaster.warning(LL.messages.insecureContext());
    }
  }, []);

  return {
    writeToClipboard,
  };
};
