import './style.scss';

import parse from 'html-react-parser';
import { isUndefined } from 'lodash-es';
import { ReactNode, useEffect, useMemo } from 'react';
import QRCode from 'react-qr-code';
import { useNavigate } from 'react-router';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { ActionButton } from '../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { ExpandableCard } from '../../../../shared/defguard-ui/components/Layout/ExpandableCard/ExpandableCard';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import useApi from '../../../../shared/hooks/useApi';
import { useClipboard } from '../../../../shared/hooks/useClipboard';
import { useAddDevicePageStore } from '../../hooks/useAddDevicePageStore';

const useLocalProxy = import.meta.env.DEV;

const extractProxyPort = (input: string): string | undefined => {
  try {
    const url = new URL(input);
    const port = url.port;
    const parsed = port ? parseInt(port, 10) : undefined;
    if (parsed && !isNaN(parsed)) {
      return `:${parsed}`;
    }
    return undefined;
  } catch {
    return undefined;
  }
};

export const AddDeviceTokenStep = () => {
  const { writeToClipboard } = useClipboard();
  const { LL } = useI18nContext();
  const localLL = LL.addDevicePage.steps.copyToken;
  const navigate = useNavigate();
  const { getAppInfo } = useApi();
  const setAppStore = useAppStore((s) => s.setState, shallow);
  const isAdmin = useAuthStore((s) => s.user?.is_admin);

  const userData = useAddDevicePageStore((state) => state.userData);

  const [url, token] = useAddDevicePageStore((state) => [
    state.enrollment?.url,
    state.enrollment?.token,
  ]);

  const [nextSubject, resetPage] = useAddDevicePageStore(
    (state) => [state.nextSubject, state.reset],
    shallow,
  );

  const mobileQrData = useMemo(() => {
    if (isPresent(url) && isPresent(token)) {
      let targetUrl: string;
      if (useLocalProxy) {
        const proxyPort = extractProxyPort(url) ?? '';
        targetUrl = `http://10.0.2.2${proxyPort}`;
      } else {
        targetUrl = url;
      }
      const registration = {
        token,
        url: targetUrl,
      };
      const registrationJson = JSON.stringify(registration);
      const encoded = btoa(registrationJson);
      return encoded;
    }
    return undefined;
  }, [token, url]);

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
        if (isAdmin) {
          void getAppInfo().then((response) => {
            setAppStore({
              appInfo: response,
            });
          });
        }
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
        <MessageBox message="If you have defguard client installed on a mobile device you can scan the QR below with your defguard client application." />
        {isPresent(mobileQrData) && (
          <ExpandableCard
            id="mobile-qr-code"
            title="Scan with mobile device"
            expanded
            disableExpand
          >
            <QRCode value={mobileQrData} size={250} />
          </ExpandableCard>
        )}
      </Card>
    </>
  );
};
