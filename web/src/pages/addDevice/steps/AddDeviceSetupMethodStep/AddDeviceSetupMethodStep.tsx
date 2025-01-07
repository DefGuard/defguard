import './style.scss';

import { useMutation } from '@tanstack/react-query';
import { useEffect, useRef } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import SvgDefguardNavLogo from '../../../../shared/components/svg/DefguardNavLogo';
import SvgWireguardLogo from '../../../../shared/components/svg/WireguardLogo';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { LoaderSpinner } from '../../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import useEffectOnce from '../../../../shared/helpers/useEffectOnce';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../shared/hooks/useApi';
import { externalLink } from '../../../../shared/links';
import { useAddDevicePageStore } from '../../hooks/useAddDevicePageStore';
import { AddDeviceMethod } from '../../types';
import { DeviceSetupMethodCard } from './components/DeviceSetupMethodCard/DeviceSetupMethodCard';

export const AddDeviceSetupMethodStep = () => {
  const {
    user: { startDesktopActivation },
  } = useApi();
  const { LL } = useI18nContext();
  const localLL = LL.addDevicePage.steps.setupMethod;
  const setupMethod = useAddDevicePageStore((state) => state.method);
  const methodRef = useRef(setupMethod);

  const userData = useAddDevicePageStore((state) => state.userData);
  const enterpriseSettings = useAppStore((state) => state.enterprise_settings);

  const [setPageState, next, nextSubject] = useAddDevicePageStore(
    (state) => [state.setState, state.nextStep, state.nextSubject],
    shallow,
  );

  const { isPending: isLoading, mutate } = useMutation({
    mutationFn: startDesktopActivation,
    onSuccess: (resp) => {
      next({
        enrollment: {
          url: resp.enrollment_url,
          token: resp.enrollment_token,
        },
      });
    },
  });

  useEffect(() => {
    const sub = nextSubject.subscribe(() => {
      if (methodRef.current === AddDeviceMethod.MANUAL) {
        next();
      } else {
        mutate({
          username: userData?.username as string,
          send_enrollment_notification: true,
          email: userData?.email as string,
        });
      }
    });
    return () => {
      sub.unsubscribe();
    };
  }, [nextSubject, next, userData?.username, userData?.email, methodRef, mutate]);

  useEffect(() => {
    methodRef.current = setupMethod;
  }, [setupMethod]);

  useEffect(() => {
    setPageState({ loading: isLoading });
  }, [isLoading, setPageState]);

  useEffectOnce(() => {
    if (enterpriseSettings?.only_client_activation) {
      setPageState({ method: AddDeviceMethod.DESKTOP });
      nextSubject.next();
    }
  });

  return (
    <>
      {!enterpriseSettings?.only_client_activation ? (
        <>
          <MessageBox
            type={MessageBoxType.WARNING}
            message={LL.addDevicePage.helpers.setupOpt()}
            dismissId="add-device-page-method-opt-message"
          />
          <Card shaded id="setup-method-step">
            <DeviceSetupMethodCard
              testId="choice-desktop"
              title={localLL.remote.title()}
              subtitle={localLL.remote.subTitle()}
              logo={<SvgDefguardNavLogo />}
              linkText={localLL.remote.link()}
              link={externalLink.defguardReleases}
              selected={setupMethod === AddDeviceMethod.DESKTOP}
              onSelect={() => {
                if (setupMethod !== AddDeviceMethod.DESKTOP) {
                  setPageState({ method: AddDeviceMethod.DESKTOP });
                }
              }}
            />
            <DeviceSetupMethodCard
              testId="choice-manual"
              title={localLL.manual.title()}
              subtitle={localLL.manual.subTitle()}
              logo={<SvgWireguardLogo />}
              linkText={localLL.manual.link()}
              link={externalLink.wireguard.download}
              selected={setupMethod === AddDeviceMethod.MANUAL}
              onSelect={() => {
                if (setupMethod !== AddDeviceMethod.MANUAL) {
                  setPageState({ method: AddDeviceMethod.MANUAL });
                }
              }}
            />
          </Card>
        </>
      ) : (
        <div id="spinner-box">
          <LoaderSpinner size={80} />
        </div>
      )}
    </>
  );
};
