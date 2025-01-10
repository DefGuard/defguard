import './styles.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useBreakpoint } from 'use-breakpoint';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../../shared/components/svg/IconCheckmarkWhite';
import { deviceBreakpoints } from '../../../../../../shared/constants';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { ActivityIcon } from '../../../../../../shared/defguard-ui/components/icons/ActivityIcon/ActivityIcon';
import { ActivityIconVariant } from '../../../../../../shared/defguard-ui/components/icons/ActivityIcon/types';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { ExpandableCard } from '../../../../../../shared/defguard-ui/components/Layout/ExpandableCard/ExpandableCard';
import { Helper } from '../../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { Label } from '../../../../../../shared/defguard-ui/components/Layout/Label/Label';
import { isPresent } from '../../../../../../shared/defguard-ui/utils/isPresent';
import { useAppStore } from '../../../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../../shared/queries';
import { Settings } from '../../../../../../shared/types';
import { invalidateMultipleQueries } from '../../../../../../shared/utils/invalidateMultipleQueries';
import { useSettingsPage } from '../../../../hooks/useSettingsPage';

type FormFields = {
  license: string;
};

type LicenseErrorResponse = {
  msg: string;
};

export const LicenseSettings = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    settings: { patchSettings },
  } = useApi();

  const settings = useSettingsPage((state) => state.settings);
  const appInfo = useAppStore((s) => s.appInfo);
  const enterpriseInfo = useSettingsPage((s) => s.enterpriseInfo);

  const queryClient = useQueryClient();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);

  const licenseIconVariant = useMemo(() => {
    if (isPresent(enterpriseInfo) && !appInfo?.license_info.any_limit_exceeded) {
      return ActivityIconVariant.CONNECTED;
    }
    return ActivityIconVariant.ERROR;
  }, [appInfo?.license_info.any_limit_exceeded, enterpriseInfo]);

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

  const { mutate, isPending: isLoading } = useMutation({
    mutationFn: patchSettings,
    onSuccess: () => {
      toaster.success(LL.settingsPage.messages.editSuccess());
      invalidateMultipleQueries(queryClient, [
        [QueryKeys.FETCH_ENTERPRISE_INFO],
        [QueryKeys.FETCH_ENTERPRISE_STATUS],
        [QueryKeys.FETCH_SETTINGS],
        [QueryKeys.FETCH_APP_INFO],
      ]);
    },
    onError: (err: AxiosError) => {
      const errorResponse = err.response?.data as LicenseErrorResponse;
      toaster.error(
        `${LL.messages.error()} ${LL.messages.details()} ${errorResponse.msg}`,
      );
      console.error(err);
    },
  });

  const zodSchema = useMemo(
    () =>
      z.object({
        license: z.string(),
      }),
    [],
  );

  const defaultValues = useMemo((): FormFields => {
    return {
      license: settings?.license || '',
    };
  }, [settings?.license]);

  const { control, handleSubmit } = useForm<Settings>({
    defaultValues,
    mode: 'all',
    resolver: zodResolver(zodSchema),
  });

  const onSubmit: SubmitHandler<FormFields> = (submitted) => {
    mutate(submitted);
  };

  return (
    <section id="license-settings">
      <header>
        <h2>{LL.settingsPage.license.header()}</h2>
        <Helper>
          <p>{LL.settingsPage.license.helpers.enterpriseHeader.text()}</p>
          <a href="https://defguard.net/pricing/" target="_blank" rel="noreferrer">
            {LL.settingsPage.license.helpers.enterpriseHeader.link()}
          </a>
        </Helper>
      </header>
      <Card shaded bordered>
        <div className="controls">
          <div className="header">
            <h3>{LL.settingsPage.license.form.title()}:</h3>
            <Helper>
              <p>{LL.settingsPage.license.helpers.licenseKey.text()}</p>
              <a href="https://defguard.net/pricing/" target="_blank" rel="noreferrer">
                {LL.settingsPage.license.helpers.licenseKey.link()}
              </a>
            </Helper>
          </div>
          <Button
            form="license-form"
            text={
              breakpoint !== 'mobile'
                ? LL.settingsPage.instanceBranding.form.controls.submit()
                : undefined
            }
            icon={<IconCheckmarkWhite />}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.SAVE}
            loading={isLoading}
            type="submit"
          />
        </div>

        <div>
          <form id="license-form" onSubmit={handleSubmit(onSubmit)}>
            <FormInput
              label={LL.settingsPage.license.form.fields.key.label()}
              controller={{ control, name: 'license' }}
              placeholder={LL.settingsPage.license.form.fields.key.placeholder()}
            />
          </form>
          <ExpandableCard title={LL.settingsPage.license.licenseInfo.title()} expanded>
            {isPresent(enterpriseInfo) ? (
              <div id="license-info">
                <div>
                  <Label>
                    {LL.settingsPage.license.licenseInfo.fields.status.label()}
                  </Label>
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
      </Card>
    </section>
  );
};
