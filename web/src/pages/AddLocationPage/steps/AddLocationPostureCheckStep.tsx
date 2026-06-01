import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import actionCardImage from '../assets/gateway-setup-action-card.png';
import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { AddLocationPageStep, type AddLocationPageStepValue } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useGatewayWizardStore } from '../../GatewaySetupPage/useGatewayWizardStore';
import { Radio } from '../../../shared/defguard-ui/components/Radio/Radio';
import { useState } from 'react';
import api from '../../../shared/api/api';
import { useNavigate } from '@tanstack/react-router';
import { cloneDeep, omit } from 'lodash-es';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { ActionCard } from '../../../shared/components/ActionCard/ActionCard';
import { Checkbox } from '../../../shared/defguard-ui/components/Checkbox/Checkbox';
import { Fold } from '../../../shared/defguard-ui/components/Fold/Fold';
import { Card } from '../../../shared/components/Card/Card';
import { Helper } from '../../../shared/defguard-ui/components/Helper/Helper';
import type { ApiDevicePosture } from '../../../shared/api/types';

// TODO
const formSchema = z.object({
  keepalive_interval: z
    .number(m.form_error_required())
    .max(65535, m.form_error_port_max()),
  mtu: z.number(m.form_error_required()).min(72).max(0xffffffff),
  fwmark: z.number(m.form_error_required()).min(0).max(0xffffffff),
});

type FormFields = z.infer<typeof formSchema>;

export const AddLocationPostureCheckStep = () => {
  const locationType = useAddLocationStore((s) => s.locationType);
  const [addPostures, setAddPostures] = useState(false);
  const [showGateway, setShowGateway] = useState(false);
  const { data: postures } = useQuery({
    queryFn: api.devicePosture.getDevicePostures,
    queryKey: ['device-posture'],
  });
  const navigate = useNavigate();

  const { mutate, isPending } = useMutation({
    mutationFn: api.location.addLocation,
    meta: {
      invalidate: [['network'], ['enterprise_info']],
    },
    onSuccess: ({ data }) => {
      if (showGateway) {
        useGatewayWizardStore.getState().start({ network_id: data.id });
        navigate({ to: '/setup-gateway', replace: true }).then(() => {
          setTimeout(() => {
            useAddLocationStore.getState().reset();
          }, 100);
        });
      } else {
        navigate({ to: '/locations', replace: true }).then(() => {
          setTimeout(() => {
            useAddLocationStore.getState().reset();
          }, 100);
        });
      }
    },
  });

  // TODO
  const defaultValues = useAddLocationStore(
    useShallow(
      (s): FormFields => ({
        keepalive_interval: s.keepalive_interval,
        mtu: s.mtu,
        fwmark: s.fwmark,
      }),
    ),
  );
  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: ({ value }) => {
      let targetStep: AddLocationPageStepValue;
      if (locationType === 'regular') {
        targetStep = AddLocationPageStep.Mfa;
      } else {
        targetStep = AddLocationPageStep.ServiceLocationSettings;
      }
      useAddLocationStore.setState({
        ...value,
        activeStep: targetStep,
      });
    },
  });

  const handleSubmit = () => {
    // TODO: set postures on the store
    const storageState = cloneDeep(
      omit(useAddLocationStore.getState(), [
        'start',
        'reset',
        'activeStep',
        'locationType',
      ]),
    );
    mutate(storageState);
  }

  return (
    <WizardCard>
      <Radio
        active={!addPostures}
        onClick={() => {
          setAddPostures(false);
        }}
        text={m.add_location_postures_dont_assign()}
        // TODO
        disabled={false}
      />
      <SizedBox height={ThemeSpacing.Md} />
      <Radio
        active={addPostures}
        onClick={() => {
          setAddPostures(true);
        }}
        text={m.add_location_postures_assign()}
        // TODO
        disabled={false}
      />
      <Fold open={addPostures}>
        <SizedBox height={ThemeSpacing.Xl2} />
        {postures && <PostureSelection postures={postures} />}
      </Fold>
      <Divider spacing={ThemeSpacing.Xl2} />
      <ActionCard
        imageSrc={actionCardImage}
        title={m.add_location_firewall_gateway_activation_title()}
        subtitle={m.add_location_firewall_gateway_activation_subtitle()}
      >
        <Checkbox
          text={m.add_location_firewall_gateway_activation_checkbox()}
          active={showGateway}
          onClick={() => {
            setShowGateway((s) => !s);
          }}
        />
      </ActionCard>
          <Controls>
            <Button
              variant="outlined"
              text={m.controls_back()}
              onClick={() => {
                useAddLocationStore.setState({
                  activeStep: AddLocationPageStep.InternalVpnSettings,
                  ...form.state.values,
                });
              }}
            />
            <div className="right">
              {/*<Button
                text={m.controls_continue()}
                testId="finish"
                onClick={() => {
                  handleSubmit();
                }}
              />*/}
              <Button
                testId="create-location"
                text={m.add_location_create_location()}
                loading={isPending}
                onClick={() => {
                  handleSubmit();
                }}
              />
            </div>
          </Controls>
    </WizardCard>
  );
};

interface PostureSelectionProps {
  postures: ApiDevicePosture[],
}
const PostureSelection = ({ postures }: PostureSelectionProps) => {

  return (
    <Card>
      {postures.map((posture) => (
        <>
          <SizedBox height={ThemeSpacing.Xl2} />
          <Checkbox
            key={posture.id}
            active={false}
            text={posture.name}
            helperBlock={
              <Helper>
                <p>{m.test_placeholder_extreme()}</p>
              </Helper>
            }
            />
        </>
      ))}
    </Card>
  );
}
