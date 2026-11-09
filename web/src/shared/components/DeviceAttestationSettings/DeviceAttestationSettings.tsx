import './style.scss';
import { useSuspenseQuery } from '@tanstack/react-query';
import { m } from '../../../paraglide/messages';
import type { DeviceAttestationCounts } from '../../api/types';
import { AppText } from '../../defguard-ui/components/AppText/AppText';
import { Chip } from '../../defguard-ui/components/Chip/Chip';
import { Divider } from '../../defguard-ui/components/Divider/Divider';
import { Fold } from '../../defguard-ui/components/Fold/Fold';
import { IconKind } from '../../defguard-ui/components/Icon/icon-types';
import { InfoBanner } from '../../defguard-ui/components/InfoBanner/InfoBanner';
import { InteractiveBlock } from '../../defguard-ui/components/InteractiveBlock/InteractiveBlock';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../defguard-ui/types';
import { getDeviceAttestationStatsQueryOptions } from '../../query';

export type DeviceAttestationSettingsProps = {
  desktopAttestation: boolean;
  desktopVerification: boolean;
  mobileAttestation: boolean;
  mobileVerification: boolean;
  onDesktopAttestationChange: (value: boolean) => void;
  onDesktopVerificationChange: (value: boolean) => void;
  onMobileAttestationChange: (value: boolean) => void;
  onMobileVerificationChange: (value: boolean) => void;
};

export const DeviceAttestationSettings = ({
  desktopAttestation,
  desktopVerification,
  mobileAttestation,
  mobileVerification,
  onDesktopAttestationChange,
  onDesktopVerificationChange,
  onMobileAttestationChange,
  onMobileVerificationChange,
}: DeviceAttestationSettingsProps) => {
  const { data: stats } = useSuspenseQuery(getDeviceAttestationStatsQueryOptions);

  return (
    <div className="device-attestation-settings">
      <DeviceAttestationSection
        title={m.cmp_device_attestation_settings_desktop_title()}
        description={m.cmp_device_attestation_settings_desktop_description()}
        counts={stats.desktop}
        attestation={desktopAttestation}
        manualVerification={desktopVerification}
        onAttestationChange={onDesktopAttestationChange}
        onManualVerificationChange={onDesktopVerificationChange}
      />
      <Divider spacing={ThemeSpacing.Xl2} />
      <DeviceAttestationSection
        title={m.cmp_device_attestation_settings_mobile_title()}
        description={m.cmp_device_attestation_settings_mobile_description()}
        counts={stats.mobile}
        attestation={mobileAttestation}
        manualVerification={mobileVerification}
        onAttestationChange={onMobileAttestationChange}
        onManualVerificationChange={onMobileVerificationChange}
      />
    </div>
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
            label={m.cmp_device_attestation_settings_impact_label()}
            items={[
              {
                label: m.cmp_device_attestation_settings_impact_attested(),
                value: counts.attested_devices,
              },
              {
                label: m.cmp_device_attestation_settings_impact_not_attested(),
                value: counts.devices - counts.attested_devices,
              },
            ]}
          />
        </div>
        <SizedBox height={ThemeSpacing.Xl2} />
        <InteractiveBlock
          variant="checkbox"
          title={m.cmp_device_attestation_settings_ownership_title()}
          content={m.cmp_device_attestation_settings_ownership_description()}
          value={manualVerification}
          onClick={() => {
            onManualVerificationChange(!manualVerification);
          }}
        >
          <Fold open={manualVerification}>
            <SizedBox height={ThemeSpacing.Lg} />
            <ImpactChips
              label={m.cmp_device_attestation_settings_ownership_impact_label()}
              items={[
                {
                  label: m.cmp_device_attestation_settings_ownership_impact_verified(),
                  value: counts.verified_devices,
                },
                {
                  label: m.cmp_device_attestation_settings_ownership_impact_unverified(),
                  value: counts.devices - counts.verified_devices,
                },
              ]}
            />
            <SizedBox height={ThemeSpacing.Md} />
            <InfoBanner
              variant="warning"
              icon={IconKind.WarningOutlined}
              text={m.cmp_device_attestation_settings_ownership_warning()}
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
