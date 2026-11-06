import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import { DeviceAttestationSettings } from '../../../shared/components/DeviceAttestationSettings/DeviceAttestationSettings';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { useAddPostureCheckWizardStore } from '../useAddPostureCheckWizardStore';

export const AddPostureCheckDeviceAttestationStep = () => {
  const {
    back,
    next,
    desktopAttestation,
    desktopManualVerification,
    mobileAttestation,
    mobileManualVerification,
    setDesktopAttestation,
    setDesktopManualVerification,
    setMobileAttestation,
    setMobileManualVerification,
  } = useAddPostureCheckWizardStore(
    useShallow((s) => ({
      back: s.back,
      next: s.next,
      desktopAttestation: s.desktopAttestation,
      desktopManualVerification: s.desktopManualVerification,
      mobileAttestation: s.mobileAttestation,
      mobileManualVerification: s.mobileManualVerification,
      setDesktopAttestation: s.setDesktopAttestation,
      setDesktopManualVerification: s.setDesktopManualVerification,
      setMobileAttestation: s.setMobileAttestation,
      setMobileManualVerification: s.setMobileManualVerification,
    })),
  );

  return (
    <WizardCard className="add-posture-check-device-attestation-step">
      <DeviceAttestationSettings
        desktopAttestation={desktopAttestation}
        desktopManualVerification={desktopManualVerification}
        mobileAttestation={mobileAttestation}
        mobileManualVerification={mobileManualVerification}
        onDesktopAttestationChange={setDesktopAttestation}
        onDesktopManualVerificationChange={setDesktopManualVerification}
        onMobileAttestationChange={setMobileAttestation}
        onMobileManualVerificationChange={setMobileManualVerification}
      />
      <Controls>
        <Button text={m.controls_back()} variant="outlined" onClick={back} />
        <div className="right">
          <Button text={m.controls_continue()} onClick={next} />
        </div>
      </Controls>
    </WizardCard>
  );
};
