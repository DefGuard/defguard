import './style.scss';

import { useEffect } from 'react';
import { useNavigate } from 'react-router';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../i18n/i18n-react';
import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { ArrowSingle } from '../../shared/defguard-ui/components/icons/ArrowSingle/ArrowSingle';
import {
  ArrowSingleDirection,
  ArrowSingleSize,
} from '../../shared/defguard-ui/components/icons/ArrowSingle/types';
import { Button } from '../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../shared/defguard-ui/components/Layout/Button/types';
import { useAppStore } from '../../shared/hooks/store/useAppStore';
import { useAuthStore } from '../../shared/hooks/store/useAuthStore';
import { useEnterpriseUpgradeStore } from '../../shared/hooks/store/useEnterpriseUpgradeStore';
import useApi from '../../shared/hooks/useApi';
import { useAddDevicePageStore } from './hooks/useAddDevicePageStore';
import { AddDeviceClientConfigurationStep } from './steps/AddDeviceClientConfigurationStep/AddDeviceClientConfigurationStep';
import { AddDeviceConfigStep } from './steps/AddDeviceConfigStep/AddDeviceConfigStep';
import { AddDeviceSetupMethodStep } from './steps/AddDeviceSetupMethodStep/AddDeviceSetupMethodStep';
import { AddDeviceSetupStep } from './steps/AddDeviceSetupStep/AddDeviceSetupStep';
import { AddDeviceNavigationEvent, AddDeviceStep } from './types';

const finalSteps: AddDeviceStep[] = [
  AddDeviceStep.NATIVE_CONFIGURATION,
  AddDeviceStep.CLIENT_CONFIGURATION,
];

export const AddDevicePage = () => {
  const { LL } = useI18nContext();
  const pageLL = LL.addDevicePage;
  const navigate = useNavigate();
  const { getAppInfo } = useApi();

  const userData = useAddDevicePageStore((state) => state.userData);
  const isAdmin = useAuthStore((s) => s.user?.is_admin ?? false);
  const setAppStore = useAppStore((s) => s.setState);
  const showUpgradeToast = useEnterpriseUpgradeStore((s) => s.show);
  const currentStep = useAddDevicePageStore((state) => state.currentStep);
  const [navSubject, resetStore, setStep] = useAddDevicePageStore(
    (s) => [s.navigationSubject, s.reset, s.setStep],
    shallow,
  );

  const isFinalStep = finalSteps.includes(currentStep);

  useEffect(() => {
    if (!userData) {
      navigate('/', { replace: true });
    }
  }, [navigate, userData]);

  useEffect(() => {
    const sub = navSubject.subscribe((event) => {
      if (
        event === AddDeviceNavigationEvent.NEXT &&
        [AddDeviceStep.CLIENT_CONFIGURATION, AddDeviceStep.NATIVE_CONFIGURATION].includes(
          currentStep,
        ) &&
        userData
      ) {
        if (isAdmin) {
          void getAppInfo().then((resp) => {
            setAppStore({ appInfo: resp });
            if (resp.license_info.any_limit_exceeded) {
              showUpgradeToast();
            }
          });
        }
        navigate(userData.originRoutePath, { replace: true });
        setTimeout(() => {
          resetStore();
        }, 250);
      }
      if (event === AddDeviceNavigationEvent.BACK) {
        if (currentStep === AddDeviceStep.NATIVE_CHOOSE_METHOD) {
          setStep(AddDeviceStep.CHOOSE_METHOD);
        }
      }
    });
    return () => {
      sub.unsubscribe();
    };
  }, [
    currentStep,
    getAppInfo,
    isAdmin,
    navSubject,
    navigate,
    resetStore,
    setAppStore,
    setStep,
    showUpgradeToast,
    userData,
  ]);

  return (
    <PageContainer id="add-device-page">
      <div className="content-wrapper">
        <header>
          <h1>{pageLL.title()}</h1>
          <div className="controls">
            <Button
              className="nav-back"
              size={ButtonSize.LARGE}
              styleVariant={ButtonStyleVariant.STANDARD}
              icon={
                currentStep !== AddDeviceStep.CHOOSE_METHOD ? (
                  <ArrowSingle direction={ArrowSingleDirection.LEFT} />
                ) : undefined
              }
              text={
                currentStep === AddDeviceStep.CHOOSE_METHOD
                  ? LL.common.controls.cancel()
                  : LL.common.controls.back()
              }
              disabled={
                ![
                  AddDeviceStep.CHOOSE_METHOD,
                  AddDeviceStep.NATIVE_CHOOSE_METHOD,
                ].includes(currentStep)
              }
              onClick={() => {
                if (currentStep === AddDeviceStep.CHOOSE_METHOD) {
                  navigate(userData?.originRoutePath ?? '/', { replace: true });
                  setTimeout(() => {
                    resetStore();
                  }, 250);
                } else {
                  navSubject.next(AddDeviceNavigationEvent.BACK);
                }
              }}
            />
            <Button
              data-testid="next-step"
              size={ButtonSize.LARGE}
              styleVariant={ButtonStyleVariant.PRIMARY}
              text={isFinalStep ? LL.common.controls.finish() : LL.common.controls.next()}
              rightIcon={
                <ArrowSingle
                  direction={ArrowSingleDirection.RIGHT}
                  size={ArrowSingleSize.SMALL}
                />
              }
              onClick={() => {
                navSubject.next(AddDeviceNavigationEvent.NEXT);
              }}
            />
          </div>
        </header>
        {steps[currentStep]}
      </div>
    </PageContainer>
  );
};

const steps = {
  [AddDeviceStep.CHOOSE_METHOD]: <AddDeviceSetupMethodStep />,
  [AddDeviceStep.NATIVE_CHOOSE_METHOD]: <AddDeviceSetupStep />,
  [AddDeviceStep.NATIVE_CONFIGURATION]: <AddDeviceConfigStep />,
  [AddDeviceStep.CLIENT_CONFIGURATION]: <AddDeviceClientConfigurationStep />,
};
