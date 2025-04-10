import './styles.scss';

import { useMemo } from 'react';
import { Control } from 'react-hook-form';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { ActivityIcon } from '../../../../../../shared/defguard-ui/components/icons/ActivityIcon/ActivityIcon';
import { ActivityIconVariant } from '../../../../../../shared/defguard-ui/components/icons/ActivityIcon/types';
import { ExpandableCard } from '../../../../../../shared/defguard-ui/components/Layout/ExpandableCard/ExpandableCard';
import { Helper } from '../../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { Label } from '../../../../../../shared/defguard-ui/components/Layout/Label/Label';
import { isPresent } from '../../../../../../shared/defguard-ui/utils/isPresent';
import { useAppStore } from '../../../../../../shared/hooks/store/useAppStore';
import { useSettingsPage } from '../../../../hooks/useSettingsPage';
import { FormFields } from '../GlobalSettingsForm/GlobalSettingsForm';

export const LicenseSettings = ({ control }: { control: Control<FormFields> }) => {
  const { LL } = useI18nContext();
  const appInfo = useAppStore((s) => s.appInfo);
  const enterpriseInfo = useSettingsPage((s) => s.enterpriseInfo);

  const licenseIconVariant = useMemo(() => {
    if (
      isPresent(enterpriseInfo) &&
      !enterpriseInfo.limits_exceeded &&
      !enterpriseInfo.expired
    ) {
      return ActivityIconVariant.CONNECTED;
    }
    return ActivityIconVariant.ERROR;
  }, [enterpriseInfo]);

  const statusText = useMemo(() => {
    if (!isPresent(enterpriseInfo)) {
      return LL.settingsPage.license.licenseInfo.status.noLicense();
    }
    if (enterpriseInfo.expired) {
      return LL.settingsPage.license.licenseInfo.status.expired();
    }
    if (appInfo?.license_info.any_limit_exceeded) {
      return LL.settingsPage.license.licenseInfo.status.limitsExceeded();
    }
    return LL.settingsPage.license.licenseInfo.status.active();
  }, [
    LL.settingsPage.license.licenseInfo.status,
    appInfo?.license_info.any_limit_exceeded,
    enterpriseInfo,
  ]);

  return (
    <div id="license-settings">
      <div className="helper-row subsection-header">
        <h3>{LL.settingsPage.license.header()}</h3>
        <Helper>
          <p>{LL.settingsPage.license.helpers.enterpriseHeader.text()}</p>
          <a href="https://defguard.net/pricing/" target="_blank" rel="noreferrer">
            {LL.settingsPage.license.helpers.enterpriseHeader.link()}
          </a>
        </Helper>
      </div>
      <div>
        <FormInput
          label={LL.settingsPage.license.form.fields.key.label()}
          controller={{ control, name: 'license' }}
          placeholder={LL.settingsPage.license.form.fields.key.placeholder()}
        />
        <ExpandableCard title={LL.settingsPage.license.licenseInfo.title()} expanded>
          {isPresent(enterpriseInfo) ? (
            <div id="license-info">
              <div>
                <Label>{LL.settingsPage.license.licenseInfo.fields.status.label()}</Label>
                <div className="license-status">
                  <ActivityIcon status={licenseIconVariant} />
                  <p>{statusText}</p>
                  {enterpriseInfo.subscription ? (
                    <Helper>
                      {LL.settingsPage.license.licenseInfo.fields.status.subscriptionHelper()}
                    </Helper>
                  ) : null}
                </div>
              </div>
              <div>
                <Label>{LL.settingsPage.license.licenseInfo.fields.type.label()}</Label>
                <div className="with-helper">
                  <p>
                    {enterpriseInfo.subscription
                      ? LL.settingsPage.license.licenseInfo.types.subscription.label()
                      : LL.settingsPage.license.licenseInfo.types.offline.label()}
                  </p>
                  <Helper>
                    {enterpriseInfo.subscription
                      ? LL.settingsPage.license.licenseInfo.types.subscription.helper()
                      : LL.settingsPage.license.licenseInfo.types.offline.helper()}
                  </Helper>
                </div>
              </div>
              <div>
                <Label>
                  {LL.settingsPage.license.licenseInfo.fields.validUntil.label()}
                </Label>
                <p>
                  {enterpriseInfo.valid_until
                    ? new Date(enterpriseInfo.valid_until).toLocaleString()
                    : '-'}
                </p>
              </div>
            </div>
          ) : (
            <>
              <p id="no-license">
                {LL.settingsPage.license.licenseInfo.status.noLicense()}
              </p>
            </>
          )}
        </ExpandableCard>
      </div>
    </div>
  );
};
