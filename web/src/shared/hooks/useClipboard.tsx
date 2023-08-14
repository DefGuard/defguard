import { useCallback } from 'react';

import { useI18nContext } from '../../i18n/i18n-react';
import { useToaster } from './useToaster';

export const useClipboard = () => {
  const { LL } = useI18nContext();

  const toaster = useToaster();

  const writeToClipboard = useCallback(
    async (content: string, customMessage?: string) => {
      if (window.isSecureContext) {
        try {
          await navigator.clipboard.writeText(content);
          if (customMessage) {
            toaster.success(customMessage);
          } else {
            toaster.success(LL.messages.clipboard.success());
          }
        } catch (e) {
          toaster.error(LL.messages.clipboard.error());
          console.error(e);
        }
      } else {
        toaster.warning(LL.messages.insecureContext());
      }
    },
    [LL.messages, toaster],
  );

  return {
    writeToClipboard,
  };
};
