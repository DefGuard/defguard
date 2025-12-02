import { useCallback } from 'react';
import { m } from '../../../paraglide/messages';
import {
  LocationServiceMode,
  type LocationServiceModeValue,
} from '../../../shared/api/types';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { RadioBlock } from '../../../shared/defguard-ui/components/RadioBlock/RadioBlock';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { AddLocationPageStep } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';

export const AddLocationServiceStep = () => {
  const currentValue = useAddLocationStore((s) => s.service_location_mode);

  const handleChange = useCallback((value: LocationServiceModeValue) => {
    useAddLocationStore.setState({
      service_location_mode: value,
    });
  }, []);

  return (
    <WizardCard>
      <RadioBlock
        value={currentValue === LocationServiceMode.Prelogon}
        onClick={() => {
          handleChange(LocationServiceMode.Prelogon);
        }}
        title="Pre-logon connection."
        content="The VPN connects at system boot and disconnects after user login, useful for one-time authentication with external identity providers like Active Directory."
      />
      <SizedBox height={ThemeSpacing.Xl} />
      <RadioBlock
        value={currentValue === LocationServiceMode.Alwayson}
        onClick={() => {
          handleChange(LocationServiceMode.Alwayson);
        }}
        title="Always on connection"
        content="Always-on: the VPN connects at system boot and stays active until the network, client, or mode changes. Use when constant access is needed."
      />
      <SizedBox height={ThemeSpacing.Xl} />
      <Divider />
      <ModalControls
        submitProps={{
          text: m.controls_continue(),
          onClick: () => {
            useAddLocationStore.setState({
              activeStep: AddLocationPageStep.AccessControl,
            });
          },
        }}
      >
        <Button
          variant="outlined"
          text={m.controls_back()}
          onClick={() => {
            useAddLocationStore.setState({
              activeStep: AddLocationPageStep.NetworkSettings,
            });
          }}
        />
      </ModalControls>
    </WizardCard>
  );
};
