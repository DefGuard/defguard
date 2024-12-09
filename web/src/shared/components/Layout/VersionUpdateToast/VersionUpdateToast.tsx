import './style.scss';

import dayjs from 'dayjs';
import { useCallback, useEffect } from 'react';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { ToastOptions } from '../../../defguard-ui/components/Layout/ToastManager/Toast/types';
import { useToastsStore } from '../../../defguard-ui/hooks/toasts/useToastStore';
import { useUpdatesStore } from '../../../hooks/store/useUpdatesStore';

// eslint-disable-next-line @typescript-eslint/no-unused-vars
export const VersionUpdateToast = ({ id }: ToastOptions) => {
  const removeToast = useToastsStore((s) => s.removeToast);
  const updateData = useUpdatesStore((s) => s.update);
  const dismissal = useUpdatesStore((s) => s.dismissal);
  const setUpdateStore = useUpdatesStore((s) => s.setStore);
  const { LL } = useI18nContext();

  const closeToast = useCallback(() => {
    removeToast(id);
  }, [id, removeToast]);

  const handleOpenModal = () => {
    setUpdateStore({ modalVisible: true });
    closeToast();
  };

  const handleDismiss = () => {
    if (updateData) {
      setUpdateStore({
        dismissal: {
          dismissedAt: dayjs.utc().toISOString(),
          version: updateData.version,
        },
      });
      closeToast();
    }
  };

  useEffect(() => {
    if (dismissal && dismissal.version === updateData?.version) {
      closeToast();
    }
  }, [closeToast, dismissal, updateData?.version]);

  if (!updateData) return null;

  return (
    <div className="update-toaster">
      <div className="top">
        <p>
          {LL.modals.updatesNotificationToaster.title({
            version: updateData.version,
          })}
        </p>
        {updateData.critical && (
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="12"
            height="12"
            viewBox="0 0 12 12"
            fill="none"
          >
            <path
              d="M6 12C9.31371 12 12 9.31371 12 6C12 2.68629 9.31371 0 6 0C2.68629 0 0 2.68629 0 6C0 9.31371 2.68629 12 6 12Z"
              style={{
                fill: 'var(--surface-alert-primary)',
              }}
            />
            <path
              d="M6.72201 7.253H5.28001L5.05301 2H6.95301L6.72201 7.253ZM4.99601 8.892C4.99487 8.77035 5.01905 8.6498 5.06701 8.538C5.11254 8.43037 5.18076 8.33384 5.26701 8.255C5.35854 8.17389 5.46433 8.11049 5.57901 8.068C5.707 8.02074 5.84258 7.99735 5.97901 7.999C6.11544 7.99735 6.25102 8.02074 6.37901 8.068C6.4953 8.10933 6.60283 8.17208 6.69601 8.253C6.78226 8.33184 6.85048 8.42837 6.89601 8.536C6.94397 8.6478 6.96815 8.76835 6.96701 8.89C6.96815 9.01165 6.94397 9.1322 6.89601 9.244C6.85048 9.35163 6.78226 9.44816 6.69601 9.527C6.60448 9.60811 6.49869 9.67151 6.38401 9.714C6.25602 9.76126 6.12044 9.78465 5.98401 9.783C5.84758 9.78465 5.712 9.76126 5.58401 9.714C5.46933 9.67151 5.36354 9.60811 5.27201 9.527C5.18576 9.44816 5.11754 9.35163 5.07201 9.244C5.0226 9.13319 4.99672 9.01333 4.99601 8.892Z"
              fill="white"
            />
          </svg>
        )}
        <a href={updateData.releaseLink} target="_blank" rel="noreferrer noopener">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="22"
            height="22"
            viewBox="0 0 22 22"
            fill="none"
          >
            <path
              d="M18 13V17C18 17.5523 17.5523 18 17 18H5C4.44772 18 4 17.5523 4 17V13C4 12.4477 4.44772 12 5 12C5.55229 12 6 12.4477 6 13V16H16V13C16 12.4477 16.4477 12 17 12C17.5523 12 18 12.4477 18 13Z"
              style={{ fill: 'var(--surface-icon-primary)' }}
            />
            <path
              d="M8.87117 7.70711C8.48065 7.31658 7.84748 7.31658 7.45696 7.70711C7.06643 8.09763 7.06643 8.7308 7.45696 9.12132L10.2854 11.9497C10.5015 12.1659 10.792 12.2624 11.0745 12.2393C11.3598 12.2652 11.654 12.1689 11.8724 11.9505L14.7009 9.12208C15.0914 8.73156 15.0914 8.0984 14.7009 7.70787C14.3103 7.31735 13.6772 7.31735 13.2866 7.70787L12 8.99451V5C12 4.44772 11.5523 4 11 4C10.4477 4 10 4.44772 10 5V8.83594L8.87117 7.70711Z"
              style={{ fill: 'var(--surface-icon-primary)' }}
            />
          </svg>
        </a>
      </div>
      <div className="bottom">
        <button
          onClick={() => {
            handleOpenModal();
          }}
        >
          {LL.modals.updatesNotificationToaster.controls.more()}
        </button>
        <button
          onClick={() => {
            handleDismiss();
          }}
        >
          {LL.common.controls.dismiss()}
        </button>
      </div>
    </div>
  );
};
