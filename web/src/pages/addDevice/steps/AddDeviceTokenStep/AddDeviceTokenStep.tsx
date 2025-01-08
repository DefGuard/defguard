import './style.scss';

import parse from 'html-react-parser';
import { isUndefined } from 'lodash-es';
import { ReactNode, useEffect, useMemo } from 'react';
import { useNavigate } from 'react-router';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { useUpgradeLicenseModal } from '../../../../shared/components/Layout/UpgradeLicenseModal/store';
import { UpgradeLicenseModalVariant } from '../../../../shared/components/Layout/UpgradeLicenseModal/types';
import { ActionButton } from '../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { ExpandableCard } from '../../../../shared/defguard-ui/components/Layout/ExpandableCard/ExpandableCard';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../shared/hooks/useApi';
import { useClipboard } from '../../../../shared/hooks/useClipboard';
import { useAddDevicePageStore } from '../../hooks/useAddDevicePageStore';

export const AddDeviceTokenStep = () => {
  const { writeToClipboard } = useClipboard();
  const { LL } = useI18nContext();
  const localLL = LL.addDevicePage.steps.copyToken;
  const navigate = useNavigate();
  const { getAppInfo } = useApi();
  const setAppStore = useAppStore((s) => s.setState, shallow);
  const openUpgradeLicenseModal = useUpgradeLicenseModal((s) => s.open, shallow);

  const userData = useAddDevicePageStore((state) => state.userData);

  const [url, token] = useAddDevicePageStore((state) => [
    state.enrollment?.url,
    state.enrollment?.token,
  ]);

  const [nextSubject, resetPage] = useAddDevicePageStore(
    (state) => [state.nextSubject, state.reset],
    shallow,
  );

  const tokenActions = useMemo(
    (): ReactNode[] => [
      <ActionButton
        variant={ActionButtonVariant.COPY}
        disabled={isUndefined(token)}
        onClick={() => {
          if (token) {
            void writeToClipboard(token);
          }
        }}
        key={0}
      />,
    ],
    [token, writeToClipboard],
  );

  const urlActions = useMemo(
    (): ReactNode[] => [
      <ActionButton
        variant={ActionButtonVariant.COPY}
        disabled={isUndefined(url)}
        onClick={() => {
          if (url) {
            void writeToClipboard(url);
          }
        }}
        key={0}
      />,
    ],
    [url, writeToClipboard],
  );

  useEffect(() => {
    const sub = nextSubject.subscribe(() => {
      if (userData) {
        void getAppInfo().then((response) => {
          setAppStore({
            appInfo: response,
          });
          if (response.license_info.limits_exceeded.device) {
            openUpgradeLicenseModal({
              modalVariant: response.license_info.enterprise
                ? UpgradeLicenseModalVariant.LICENSE_LIMIT
                : UpgradeLicenseModalVariant.ENTERPRISE_NOTICE,
            });
          }
        });
        setTimeout(() => {
          resetPage();
        }, 1000);
        navigate(userData.originRoutePath, { replace: true });
      }
    });
    return () => {
      sub.unsubscribe();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [nextSubject, userData]);

  return (
    <>
      <MessageBox
        type={MessageBoxType.WARNING}
        message={parse(LL.addDevicePage.helpers.client())}
      />
      <Card id="add-device-token-step" shaded>
        <h2>{localLL.title()}</h2>
        <ExpandableCard title={localLL.urlCardTitle()} actions={urlActions} expanded>
          <p>{url}</p>
        </ExpandableCard>
        <ExpandableCard title={localLL.tokenCardTitle()} actions={tokenActions} expanded>
          <p>{token}</p>
        </ExpandableCard>
      </Card>
    </>
  );
};
