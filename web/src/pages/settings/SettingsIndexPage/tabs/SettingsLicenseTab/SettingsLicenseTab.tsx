import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { m } from '../../../../../paraglide/messages';
import type { LicenseInfo } from '../../../../../shared/api/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { DescriptionBlock } from '../../../../../shared/components/DescriptionBlock/DescriptionBlock';
import { SettingsCard } from '../../../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../../../shared/components/SettingsLayout/SettingsLayout';
import { AppText } from '../../../../../shared/defguard-ui/components/AppText/AppText';
import { Badge } from '../../../../../shared/defguard-ui/components/Badge/Badge';
import { Button } from '../../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../../shared/defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../../shared/defguard-ui/utils/isPresent';
import { openModal } from '../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../shared/hooks/modalControls/modalTypes';
import {
  getLicenseInfoQueryOptions,
  getSettingsQueryOptions,
} from '../../../../../shared/query';
import { getLicenseState, type LicenseState } from '../../../../../shared/utils/license';
import { SettingsLicenseBusinessUpsellSection } from './components/SettingsLicenseBusinessUpsellSection/SettingsLicenseBusinessUpsellSection';
import { SettingsLicenseExpiredNotice } from './components/SettingsLicenseExpiredNotice/SettingsLicenseExpiredNotice';
import { SettingsLicenseInfoSection } from './components/SettingsLicenseInfoSection/SettingsLicenseInfoSection';
import { SettingsLicenseNoLicenseSection } from './components/SettingsLicenseNoLicenseSection/SettingsLicenseNoLicenseSection';
import { SettingsLicenseModal } from './modals/SettingsLicenseModal/SettingsLicenseModal';

export const SettingsLicenseTab = () => {
  const { data: licenseInfo } = useQuery(getLicenseInfoQueryOptions);
  const { data: settings } = useQuery(getSettingsQueryOptions);

  const licenseState = getLicenseState(licenseInfo);

  return (
    <SettingsLayout id="settings-license-tab">
      <SettingsHeader
        icon="credit-card"
        title={m.settings_license_title()}
        subtitle={m.settings_license_subtitle()}
      />
      {isPresent(settings) && (
        <SettingsCard className="license-main-card">
          {isPresent(licenseInfo) &&
            isPresent(licenseState) &&
            licenseState !== 'noLicense' && (
              <SettingsLicenseInfoSection
                licenseInfo={licenseInfo}
                licenseState={licenseState}
              />
            )}
          {!isPresent(licenseInfo) && (
            <div className="empty-plan">
              <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgNeutral}>
                {m.settings_license_current_plan()}
              </AppText>
              <SizedBox height={ThemeSpacing.Sm} />
              <Badge variant="neutral" text={m.settings_license_no_plan()} />
              <Divider spacing={ThemeSpacing.Xl} />
            </div>
          )}
          <DescriptionBlock title={m.settings_license_key_title()}>
            <p>{m.settings_license_key_description()}</p>
          </DescriptionBlock>
          <Controls>
            <div className="left">
              <Button
                variant="primary"
                text={
                  (settings.license?.length ?? 0) > 0
                    ? m.settings_license_edit_button()
                    : m.settings_license_enter_button()
                }
                onClick={() => {
                  openModal(ModalName.SettingsLicense, {
                    license: settings.license,
                  });
                }}
              />
            </div>
          </Controls>
        </SettingsCard>
      )}
      <LicenseSection state={licenseState} licenseInfo={licenseInfo} />
      <SettingsLicenseModal />
    </SettingsLayout>
  );
};

const LicenseSection = ({
  licenseInfo,
  state,
}: {
  licenseInfo: LicenseInfo | null | undefined;
  state: LicenseState | null;
}) => {
  if (state === null || state === 'validEnterprise') {
    return null;
  }

  return (
    <>
      <SizedBox height={ThemeSpacing.Xl} />
      {state === 'noLicense' && <SettingsLicenseNoLicenseSection />}
      {state === 'gracePeriod' && isPresent(licenseInfo) && (
        <SettingsLicenseExpiredNotice licenseInfo={licenseInfo} state="gracePeriod" />
      )}
      {state === 'expiredLicense' && isPresent(licenseInfo) && (
        <SettingsLicenseExpiredNotice licenseInfo={licenseInfo} state="expiredLicense" />
      )}
      {state === 'validBusiness' && <SettingsLicenseBusinessUpsellSection />}
    </>
  );
};
