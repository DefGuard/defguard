import { useSuspenseQuery } from '@tanstack/react-query';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../paraglide/messages';
import type { DeviceAttestationCounts } from '../../../shared/api/types';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Chip } from '../../../shared/defguard-ui/components/Chip/Chip';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { Fold } from '../../../shared/defguard-ui/components/Fold/Fold';
import { IconKind } from '../../../shared/defguard-ui/components/Icon/icon-types';
import { InfoBanner } from '../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { InteractiveBlock } from '../../../shared/defguard-ui/components/InteractiveBlock/InteractiveBlock';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../shared/defguard-ui/types';
import { getDeviceAttestationStatsQueryOptions } from '../../../shared/query';
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
  const { data: stats } = useSuspenseQuery(getDeviceAttestationStatsQueryOptions);

  return (
    <WizardCard className="add-posture-check-device-attestation-step">
      <div className="device-attestation-track">
        <DeviceAttestationSection
          title={m.posture_checks_wizard_device_attestation_desktop_title()}
          description={m.posture_checks_wizard_device_attestation_desktop_description()}
          counts={stats.desktop}
          attestation={desktopAttestation}
          manualVerification={desktopManualVerification}
          onAttestationChange={setDesktopAttestation}
          onManualVerificationChange={setDesktopManualVerification}
        />
        <Divider spacing={ThemeSpacing.Xl2} />
        <DeviceAttestationSection
          title={m.posture_checks_wizard_device_attestation_mobile_title()}
          description={m.posture_checks_wizard_device_attestation_mobile_description()}
          counts={stats.mobile}
          attestation={mobileAttestation}
          manualVerification={mobileManualVerification}
          onAttestationChange={setMobileAttestation}
          onManualVerificationChange={setMobileManualVerification}
        />
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

type DeviceAttestationSectionProps = {
  title: string;
  description: string;
  counts: DeviceAttestationCounts;
  attestation: boolean;
  manualVerification: boolean;
  onAttestationChange: (value: boolean) => void;
  onManualVerificationChange: (value: boolean) => void;
};

const DeviceAttestationSection = ({
  title,
  description,
  counts,
  attestation,
  manualVerification,
  onAttestationChange,
  onManualVerificationChange,
}: DeviceAttestationSectionProps) => {
  return (
    <>
      <InteractiveBlock
        variant="toggle"
        title={title}
        content={description}
        value={attestation}
        onClick={() => {
          onAttestationChange(!attestation);
        }}
      />
      <Fold open={attestation}>
        <SizedBox height={ThemeSpacing.Lg} />
        <div className="attestation-impact">
          <ImpactChips
            label={m.posture_checks_wizard_device_attestation_impact_label()}
            items={[
              {
                label: m.posture_checks_wizard_device_attestation_impact_attested(),
                value: counts.attested_devices,
              },
              {
                label: m.posture_checks_wizard_device_attestation_impact_not_attested(),
                value: counts.devices - counts.attested_devices,
              },
            ]}
          />
        </div>
        <SizedBox height={ThemeSpacing.Xl2} />
        <InteractiveBlock
          variant="checkbox"
          title={m.posture_checks_wizard_device_attestation_ownership_title()}
          content={m.posture_checks_wizard_device_attestation_ownership_description()}
          value={manualVerification}
          onClick={() => {
            onManualVerificationChange(!manualVerification);
          }}
        >
          <Fold open={manualVerification}>
            <SizedBox height={ThemeSpacing.Lg} />
            <ImpactChips
              label={m.posture_checks_wizard_device_attestation_ownership_impact_label()}
              items={[
                {
                  label:
                    m.posture_checks_wizard_device_attestation_ownership_impact_verified(),
                  value: counts.verified_devices,
                },
                {
                  label:
                    m.posture_checks_wizard_device_attestation_ownership_impact_unverified(),
                  value: counts.devices - counts.verified_devices,
                },
              ]}
            />
            <SizedBox height={ThemeSpacing.Md} />
            <InfoBanner
              variant="warning"
              icon={IconKind.WarningOutlined}
              text={m.posture_checks_wizard_device_attestation_ownership_warning()}
            />
          </Fold>
        </InteractiveBlock>
      </Fold>
    </>
  );
};

type ImpactChipsProps = {
  label: string;
  items: Array<{ label: string; value: number }>;
};

const ImpactChips = ({ label, items }: ImpactChipsProps) => {
  return (
    <div className="impact-chips">
      <AppText font={TextStyle.TBodyXs500} color={ThemeVariable.FgMuted}>
        {label}
      </AppText>
      <div className="chips">
        {items.map((item) => (
          <Chip key={item.label} text={`${item.label} • ${item.value}`} />
        ))}
      </div>
    </div>
  );
};
