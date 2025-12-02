import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { cloneDeep, omit } from 'lodash-es';
import { useCallback, useState } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
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
  const navigate = useNavigate();

  const { mutate, isPending } = useMutation({
    mutationFn: api.location.addLocation,
    meta: {
      invalidate: ['network'],
    },
    onSuccess: () => {
      navigate({ to: '/locations', replace: true }).then(() => {
        setTimeout(() => {
          useAddLocationStore.getState().reset();
        }, 100);
      });
    },
  });

  const saveChanges = useCallback((value: Choice) => {
    const enabled = value !== 'disable';
    const allowed = value === 'enabled-allowed';
    useAddLocationStore.setState({
      acl_enabled: enabled,
      acl_default_allow: allowed,
    });
  }, []);

  const handleSubmit = () => {
    const enabled = state !== 'disable';
    const allowed = state === 'enabled-allowed';
    const storageState = cloneDeep(
      omit(useAddLocationStore.getState(), [
        'start',
        'reset',
        'activeStep',
        'locationType',
      ]),
    );
    storageState.acl_enabled = enabled;
    storageState.acl_default_allow = allowed;
    mutate(storageState);
  };

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
          loading: isPending,
          onClick: () => {
            handleSubmit();
          },
        }}
      >
        <Button
          variant="outlined"
          text={m.controls_back()}
          disabled={isPending}
          onClick={() => {
            saveChanges(state);
            useAddLocationStore.setState({
              activeStep: AddLocationPageStep.NetworkSettings,
            });
          }}
        />
      </ModalControls>
    </WizardCard>
  );
};
