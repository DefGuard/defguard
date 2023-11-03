import './style.scss';

import { ReactNode, useEffect, useMemo } from 'react';
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
import { useAddDevicePageStore } from './hooks/useAddDevicePageStore';
import { AddDeviceSetupMethodStep } from './steps/AddDeviceSetupMethodStep/AddDeviceSetupMethodStep';
import { AddDeviceMethod } from './types';

export const AddDevicePage = () => {
  const { LL } = useI18nContext();
  const pageLL = LL.addDevicePage;
  const navigate = useNavigate();

  const userData = useAddDevicePageStore((state) => state.userData);

  const [currentStep, setupMethod] = useAddDevicePageStore(
    (state) => [state.currentStep, state.method],
    shallow,
  );

  const nextSubject = useAddDevicePageStore((state) => state.nextSubject);

  const steps = useMemo((): ReactNode[] => {
    if (setupMethod === AddDeviceMethod.MANUAL) {
      return manualSteps;
    }
    return desktopSteps;
  }, [setupMethod]);

  const stepsMax = useMemo(
    () => (setupMethod === AddDeviceMethod.MANUAL ? 2 : 1),
    [setupMethod],
  );

  useEffect(() => {
    if (!userData) {
      navigate('/', { replace: true });
    }
  }, [navigate, userData]);

  return (
    <PageContainer id="add-device-page">
      <div className="content-wrapper">
        <header>
          <h1>{pageLL.title()}</h1>
          <div className="controls">
            <Button
              size={ButtonSize.LARGE}
              styleVariant={ButtonStyleVariant.STANDARD}
              text={LL.common.controls.cancel()}
              onClick={() => {
                navigate('/', { replace: true });
              }}
            />
            <Button
              size={ButtonSize.LARGE}
              styleVariant={ButtonStyleVariant.PRIMARY}
              text={
                currentStep === stepsMax
                  ? LL.common.controls.finish()
                  : LL.common.controls.next()
              }
              rightIcon={
                <ArrowSingle
                  direction={ArrowSingleDirection.RIGHT}
                  size={ArrowSingleSize.SMALL}
                />
              }
              onClick={() => nextSubject.next()}
            />
          </div>
        </header>
        {currentStep === 0 && <AddDeviceSetupMethodStep />}
        {currentStep !== 0 && steps[currentStep]}
      </div>
    </PageContainer>
  );
};

const manualSteps: ReactNode[] = [];
const desktopSteps: ReactNode[] = [];
