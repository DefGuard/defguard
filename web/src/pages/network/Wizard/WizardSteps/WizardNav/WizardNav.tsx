import './style.scss';

import React, { useEffect, useMemo } from 'react';
import { useNavigate } from 'react-router';
import useBreakpoint from 'use-breakpoint';
import shallow from 'zustand/shallow';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/components/layout/Button/Button';
import SvgIconArrowGrayLeft from '../../../../../shared/components/svg/IconArrowGrayLeft';
import SvgIconArrowGrayRight from '../../../../../shared/components/svg/IconArrowGrayRight';
import { deviceBreakpoints } from '../../../../../shared/constants';
import { useWizardStore } from '../store';

interface Props {
  title: string;
  currentStep: number;
  steps: number;
}

const WizardNav: React.FC<Props> = ({ title, currentStep, steps }) => {
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const navigate = useNavigate();
  const [formSubmissionSubject, proceedWizardSubject] = useWizardStore(
    (state) => [state.formSubmissionSubject, state.proceedWizardSubject],
    shallow
  );
  // TODO: cleanup
  // const setAppStore = useAppStore((state) => state.setAppStore);

  const getClassName = useMemo(() => {
    const res = ['controls'];
    return res.join(' ');
  }, []);

  // TODO: cleanup
  // const {
  //   network: { addNetwork, editNetwork },
  // } = useApi();
  // const queryClient = useQueryClient();

  // const addNetworkMutation = useMutation(
  //   (networkData: Network) => addNetwork(networkData),
  //   {
  //     onSuccess: () => {
  //       queryClient.invalidateQueries([QueryKeys.FETCH_NETWORKS]);
  //       resetStore({ editMode: false });
  //       setAppStore({ wizardCompleted: true });
  //       navigate('/admin/overview', { state: { created: true } });
  //     },
  //   }
  // );

  // const editNetworkMutation = useMutation(
  //   (networkData: Network) => editNetwork(networkData),
  //   {
  //     onSuccess: () => {
  //       queryClient.invalidateQueries([QueryKeys.FETCH_NETWORKS]);
  //       resetStore({ editMode: false });
  //       navigate('/admin/overview');
  //     },
  //   }
  // );

  useEffect(() => {
    const sub = proceedWizardSubject.subscribe(() => {
      if (currentStep === steps) {
        // TODO: remove this if branch
        // // Finish clicked
        // const currentNetwork = networkObserver?.getValue();
        // if (currentNetwork) {
        //   if (editMode) {
        //     editNetworkMutation.mutate(wizardToApiNetwork(currentNetwork));
        //   } else {
        //     addNetworkMutation.mutate(wizardToApiNetwork(currentNetwork));
        //   }
        // }
      } else {
        navigate(`../${currentStep + 1}`);
      }
    });
    return () => sub.unsubscribe();
  }, [
    // addNetworkMutation,
    currentStep,
    // editMode,
    // editNetworkMutation,
    navigate,
    // networkObserver,
    proceedWizardSubject,
    steps,
  ]);

  return (
    <nav className="wizard-nav">
      <div>
        {breakpoint === 'desktop' && <h1>{title}</h1>}
        <div className={getClassName}>
          <Button
            data-test="back"
            onClick={() =>
              navigate(currentStep - 1 === 0 ? '' : `../${currentStep - 1}`)
            }
            size={ButtonSize.SMALL}
            text="Back"
            icon={<SvgIconArrowGrayLeft />}
            disabled={currentStep === 1}
          />
          <Button
            data-test="next"
            text={currentStep === steps ? 'Finish' : 'Next'}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.PRIMARY}
            icon={currentStep !== steps ? <SvgIconArrowGrayRight /> : null}
            onClick={() => formSubmissionSubject.next(currentStep)}
          />
        </div>
      </div>
    </nav>
  );
};
export default WizardNav;
