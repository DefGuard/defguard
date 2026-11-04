import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { Fold } from '../../../shared/defguard-ui/components/Fold/Fold';
import { InteractiveBlock } from '../../../shared/defguard-ui/components/InteractiveBlock/InteractiveBlock';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useAddPostureCheckWizardStore } from '../useAddPostureCheckWizardStore';

export const AddPostureCheckDeviceAttestationStep = () => {
  const back = useAddPostureCheckWizardStore((s) => s.back);
  const next = useAddPostureCheckWizardStore((s) => s.next);
  const desktopAttestation = useAddPostureCheckWizardStore((s) => s.desktopAttestation);
  const desktopManualVerification = useAddPostureCheckWizardStore(
    (s) => s.desktopManualVerification,
  );
  const mobileAttestation = useAddPostureCheckWizardStore((s) => s.mobileAttestation);
  const mobileManualVerification = useAddPostureCheckWizardStore(
    (s) => s.mobileManualVerification,
  );
  const setDesktopAttestation = useAddPostureCheckWizardStore(
    (s) => s.setDesktopAttestation,
  );
  const setDesktopManualVerification = useAddPostureCheckWizardStore(
    (s) => s.setDesktopManualVerification,
  );
  const setMobileAttestation = useAddPostureCheckWizardStore(
    (s) => s.setMobileAttestation,
  );
  const setMobileManualVerification = useAddPostureCheckWizardStore(
    (s) => s.setMobileManualVerification,
  );

  return (
    <WizardCard className="add-posture-check-device-attestation-step">
      <div className="device-attestation-track">
        <InteractiveBlock
          variant="toggle"
          title={m.posture_checks_wizard_device_attestation_desktop_title()}
          content={m.posture_checks_wizard_device_attestation_desktop_description()}
          value={desktopAttestation}
          onClick={() => {
            setDesktopAttestation(!desktopAttestation);
          }}
        >
          <Fold open={desktopAttestation}>
            <SizedBox height={ThemeSpacing.Lg} />
            <InteractiveBlock
              variant="checkbox"
              title={m.posture_checks_wizard_device_attestation_ownership_title()}
              content={m.posture_checks_wizard_device_attestation_ownership_description()}
              value={desktopManualVerification}
              onClick={() => {
                setDesktopManualVerification(!desktopManualVerification);
              }}
            />
          </Fold>
        </InteractiveBlock>
        <Divider spacing={ThemeSpacing.Xl2} />
        <InteractiveBlock
          variant="toggle"
          title={m.posture_checks_wizard_device_attestation_mobile_title()}
          content={m.posture_checks_wizard_device_attestation_mobile_description()}
          value={mobileAttestation}
          onClick={() => {
            setMobileAttestation(!mobileAttestation);
          }}
        >
          <Fold open={mobileAttestation}>
            <SizedBox height={ThemeSpacing.Lg} />
            <InteractiveBlock
              variant="checkbox"
              title={m.posture_checks_wizard_device_attestation_ownership_title()}
              content={m.posture_checks_wizard_device_attestation_ownership_description()}
              value={mobileManualVerification}
              onClick={() => {
                setMobileManualVerification(!mobileManualVerification);
              }}
            />
          </Fold>
        </InteractiveBlock>
      </div>
      <Controls>
        <Button text={m.controls_back()} variant="outlined" onClick={back} />
        <div className="right">
          <Button text={m.controls_continue()} onClick={next} />
        </div>
      </Controls>
    </WizardCard>
  );
};
