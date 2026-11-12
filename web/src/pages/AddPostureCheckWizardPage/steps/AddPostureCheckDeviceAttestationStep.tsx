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
    desktopVerification,
    mobileAttestation,
    mobileVerification,
    setDesktopAttestation,
    setDesktopVerification,
    setMobileAttestation,
    setMobileVerification,
  } = useAddPostureCheckWizardStore(
    useShallow((s) => ({
      back: s.back,
      next: s.next,
      desktopAttestation: s.desktopAttestation,
      desktopVerification: s.desktopVerification,
      mobileAttestation: s.mobileAttestation,
      mobileVerification: s.mobileVerification,
      setDesktopAttestation: s.setDesktopAttestation,
      setDesktopVerification: s.setDesktopVerification,
      setMobileAttestation: s.setMobileAttestation,
      setMobileVerification: s.setMobileVerification,
    })),
  );

  return (
    <WizardCard className="add-posture-check-device-attestation-step">
      <DeviceAttestationSettings
        desktopAttestation={desktopAttestation}
        desktopVerification={desktopVerification}
        mobileAttestation={mobileAttestation}
        mobileVerification={mobileVerification}
        onDesktopAttestationChange={setDesktopAttestation}
        onDesktopVerificationChange={setDesktopVerification}
        onMobileAttestationChange={setMobileAttestation}
        onMobileVerificationChange={setMobileVerification}
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
