import './style.scss';

import { useMutation } from '@tanstack/react-query';
import { useCallback, useEffect, useState } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { LoaderSpinner } from '../../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../shared/hooks/useApi';
import { useAddDevicePageStore } from '../../hooks/useAddDevicePageStore';
import { AddDeviceNavigationEvent, AddDeviceStep } from '../../types';
import { DeviceSetupMethodCard } from './components/DeviceSetupMethodCard/DeviceSetupMethodCard';
import { DeviceSetupMethod } from './types';

export const AddDeviceSetupMethodStep = () => {
  const {
    user: { startDesktopActivation },
  } = useApi();
  const { LL } = useI18nContext();
  const localLL = LL.addDevicePage.steps.setupMethod;

  const [setupMethod, setSetupMethod] = useState(AddDeviceStep.CLIENT_CONFIGURATION);
  const userData = useAddDevicePageStore((state) => state.userData);

  const enterpriseSettings = useAppStore((state) => state.enterprise_settings);
  const [navSubject, setPageState, setStep] = useAddDevicePageStore(
    (s) => [s.navigationSubject, s.setState, s.setStep],
    shallow,
  );

  const { mutate, isPending } = useMutation({
    mutationFn: startDesktopActivation,
    onSuccess: (resp) => {
      setStep(setupMethod, {
        clientSetup: {
          url: resp.enrollment_url,
          token: resp.enrollment_token,
        },
      });
    },
  });

  const startActivation = useCallback(() => {
    mutate({
      username: userData?.username as string,
      send_enrollment_notification: true,
      email: userData?.email as string,
    });
  }, [mutate, userData?.email, userData?.username]);

  useEffect(() => {
    const sub = navSubject.subscribe((event) => {
      if (event === AddDeviceNavigationEvent.NEXT) {
        switch (setupMethod) {
          case AddDeviceStep.NATIVE_CHOOSE_METHOD:
            setPageState({ currentStep: AddDeviceStep.NATIVE_CHOOSE_METHOD });
            break;
          case AddDeviceStep.CLIENT_CONFIGURATION:
            startActivation();
            break;
        }
      }
    });
    return () => {
      sub.unsubscribe();
    };
  }, [mutate, navSubject, setPageState, setupMethod, startActivation]);

  useEffect(() => {
    if (
      enterpriseSettings?.only_client_activation &&
      setupMethod === AddDeviceStep.NATIVE_CHOOSE_METHOD
    ) {
      setSetupMethod(AddDeviceStep.CLIENT_CONFIGURATION);
      startActivation();
    }
  }, [
    enterpriseSettings?.only_client_activation,
    setPageState,
    setupMethod,
    startActivation,
  ]);

  return (
    <>
      {!isPending ? (
        <Card shaded id="setup-method-step">
          <p className="title">{localLL.title()}</p>
          <MessageBox message={localLL.message()} />
          <div className="primary-methods">
            <DeviceSetupMethodCard
              methodType={DeviceSetupMethod.CLIENT}
              active={setupMethod === AddDeviceStep.CLIENT_CONFIGURATION}
              onClick={() => {
                setSetupMethod(AddDeviceStep.CLIENT_CONFIGURATION);
              }}
            />
            <DeviceSetupMethodCard
              methodType={DeviceSetupMethod.NATIVE_WG}
              active={setupMethod === AddDeviceStep.NATIVE_CHOOSE_METHOD}
              onClick={() => {
                setSetupMethod(AddDeviceStep.NATIVE_CHOOSE_METHOD);
              }}
            />
          </div>
        </Card>
      ) : (
        <div id="spinner-box">
          <LoaderSpinner size={80} />
        </div>
      )}
    </>
  );
};
