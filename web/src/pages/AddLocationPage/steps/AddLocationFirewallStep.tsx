import { useCallback, useState } from 'react';
import { m } from '../../../paraglide/messages';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { Radio } from '../../../shared/defguard-ui/components/Radio/Radio';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { AddLocationPageStep } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';

type Choice = 'disable' | 'enabled-allowed' | 'enabled-denied';

export const AddLocationFirewallStep = () => {
  const [state, setState] = useState<Choice>('disable');

  const saveChanges = useCallback((value: Choice) => {
    const enabled = value !== 'disable';
    const allowed = value === 'enabled-allowed';
    useAddLocationStore.setState({
      acl_enabled: enabled,
      acl_default_allow: allowed,
    });
  }, []);

  return (
    <WizardCard>
      <Radio
        active={state === 'disable'}
        onClick={() => {
          setState('disable');
        }}
        text="Disable firewall option"
      />
      <SizedBox height={ThemeSpacing.Md} />
      <Radio
        active={state === 'enabled-allowed'}
        onClick={() => {
          setState('enabled-allowed');
        }}
        text="Users/devices can access all resources unless limited by ACL rules."
      />
      <SizedBox height={ThemeSpacing.Md} />
      <Radio
        active={state === 'enabled-denied'}
        onClick={() => {
          setState('enabled-denied');
        }}
        text="All traffic not explicitly allowed by an ACL rule will be blocked."
      />
      <ModalControls
        submitProps={{
          text: m.controls_continue(),
          onClick: () => {
            saveChanges(state);
            useAddLocationStore.setState({
              activeStep: AddLocationPageStep.Mfa,
            });
          },
        }}
      >
        <Button
          variant="outlined"
          text={m.controls_back()}
          onClick={() => {
            saveChanges(state);
            useAddLocationStore.setState({
              activeStep: AddLocationPageStep.LocationAccess,
            });
          }}
        />
      </ModalControls>
    </WizardCard>
  );
};
