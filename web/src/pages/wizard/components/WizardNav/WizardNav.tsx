import './style.scss';

import { useQueryClient } from '@tanstack/react-query';
import { useEffect } from 'react';
import { useNavigate } from 'react-router';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import DefguardNoIcon from '../../../../shared/components/svg/DefguardNoIcon';
import SvgIconArrowGrayLeft from '../../../../shared/components/svg/IconArrowGrayLeft';
import SvgIconArrowGrayRight from '../../../../shared/components/svg/IconArrowGrayRight';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { Divider } from '../../../../shared/defguard-ui/components/Layout/Divider/Divider';
import { DividerDirection } from '../../../../shared/defguard-ui/components/Layout/Divider/types';
import { useAppStore } from '../../../../shared/hooks/store/useAppStore';
import { useEnterpriseUpgradeStore } from '../../../../shared/hooks/store/useEnterpriseUpgradeStore';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../shared/queries';
import { invalidateMultipleQueries } from '../../../../shared/utils/invalidateMultipleQueries';
import { useWizardStore } from '../../hooks/useWizardStore';

interface Props {
  title: string;
  lastStep: boolean;
  backDisabled?: boolean;
}

export const WizardNav = ({ title, lastStep, backDisabled = false }: Props) => {
  const { getAppInfo } = useApi();
  const setAppState = useAppStore((s) => s.setState, shallow);
  const queryClient = useQueryClient();
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const navigate = useNavigate();
  const networkPresent = useAppStore((state) => state.appInfo?.network_present);
  const [currentStep, loading] = useWizardStore(
    (state) => [state.currentStep, state.loading],
    shallow,
  );
  const [back, submitSubject, next, nextSubject, resetState] = useWizardStore(
    (state) => [
      state.perviousStep,
      state.submitSubject,
      state.nextStep,
      state.nextStepSubject,
      state.resetState,
    ],
    shallow,
  );
  const showUpgradeToast = useEnterpriseUpgradeStore((s) => s.show);

  useEffect(() => {
    const sub = nextSubject.subscribe(() => {
      if (lastStep) {
        toaster.success(LL.wizard.completed());
        resetState();
        invalidateMultipleQueries(queryClient, [[QueryKeys.FETCH_NETWORKS]]);
        void getAppInfo().then((response) => {
          setAppState({ appInfo: response });
          if (response.license_info.any_limit_exceeded) {
            showUpgradeToast();
          }
        });
        navigate('/admin/overview', { replace: true });
      } else {
        next();
      }
    });
    return () => sub?.unsubscribe();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [LL.wizard, lastStep]);

  if (!networkPresent && currentStep === 0) return null;

  return (
    <div id="wizard-nav">
      <div className="top">
        <DefguardNoIcon /> <Divider direction={DividerDirection.VERTICAL} />
        <span>{LL.wizard.navigation.top()}</span>
      </div>
      <div className="bottom">
        <h1>{title}</h1>
        <div className="controls">
          <Button
            data-testid="wizard-back"
            onClick={() => back()}
            size={ButtonSize.LARGE}
            text="Back"
            icon={<SvgIconArrowGrayLeft />}
            disabled={loading || backDisabled}
          />
          <Button
            data-testid="wizard-next"
            text={lastStep ? 'Finish' : 'Next'}
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.PRIMARY}
            icon={!lastStep ? <SvgIconArrowGrayRight /> : null}
            loading={loading}
            onClick={() => submitSubject?.next()}
          />
        </div>
      </div>
    </div>
  );
};
