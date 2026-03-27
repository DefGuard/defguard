import { useCallback } from 'react';
import { m } from '../../../paraglide/messages';
import {
  LocationServiceMode,
  type LocationServiceModeValue,
} from '../../../shared/api/types';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { InteractiveBlock } from '../../../shared/defguard-ui/components/InteractiveBlock/InteractiveBlock';
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
      <InteractiveBlock
        value={currentValue === LocationServiceMode.Prelogon}
        onClick={() => {
          handleChange(LocationServiceMode.Prelogon);
        }}
        title={m.add_location_service_prelogon_title()}
        content={m.add_location_service_prelogon_content()}
      />
      <SizedBox height={ThemeSpacing.Xl} />
      <InteractiveBlock
        value={currentValue === LocationServiceMode.Alwayson}
        onClick={() => {
          handleChange(LocationServiceMode.Alwayson);
        }}
        title={m.add_location_service_always_on_title()}
        content={m.add_location_service_always_on_content()}
      />
      <SizedBox height={ThemeSpacing.Xl} />
      <Divider />
      <Controls>
        <Button
          variant="outlined"
          text={m.controls_back()}
          onClick={() => {
            useAddLocationStore.setState({
              activeStep: AddLocationPageStep.NetworkSettings,
            });
          }}
        />
        <div className="right">
          <Button
            text={m.controls_continue()}
            testId="continue"
            onClick={() => {
              useAddLocationStore.setState({
                activeStep: AddLocationPageStep.AccessControl,
              });
            }}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
