import './styles.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import parse from 'html-react-parser';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useBreakpoint } from 'use-breakpoint';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../../shared/components/svg/IconCheckmarkWhite';
import { deviceBreakpoints } from '../../../../../../shared/constants';
import { FormCheckBox } from '../../../../../../shared/defguard-ui/components/Form/FormCheckBox/FormCheckBox';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Helper } from '../../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import SvgIconX from '../../../../../../shared/defguard-ui/components/svg/IconX';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { externalLink } from '../../../../../../shared/links';
import { QueryKeys } from '../../../../../../shared/queries';
import { invalidateMultipleQueries } from '../../../../../../shared/utils/invalidateMultipleQueries';
import { useSettingsPage } from '../../../../hooks/useSettingsPage';
import { LicenseSettings } from '../LicenseSettings/LicenseSettings';

export type FormFields = {
  instance_name: string;
  main_logo_url: string;
  nav_logo_url: string;
  openid_enabled: boolean;
  wireguard_enabled: boolean;
  worker_enabled: boolean;
  webhooks_enabled: boolean;
  license: string;
};

const defaultSettings: FormFields = {
  instance_name: 'Defguard',
  main_logo_url: '/svg/logo-defguard-white.svg',
  nav_logo_url: '/svg/defguard-nav-logo.svg',
  openid_enabled: false,
  wireguard_enabled: false,
  worker_enabled: false,
  webhooks_enabled: false,
  license: '',
};

export const GlobalSettingsForm = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    settings: { patchSettings },
  } = useApi();

  const settings = useSettingsPage((state) => state.settings);

  const queryClient = useQueryClient();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);

  const { mutate, isPending: isLoading } = useMutation({
    mutationFn: patchSettings,
    onSuccess: () => {
      const keys = [
        [QueryKeys.FETCH_ENTERPRISE_INFO],
        [QueryKeys.FETCH_ENTERPRISE_STATUS],
        [QueryKeys.FETCH_SETTINGS],
        [QueryKeys.FETCH_APP_INFO],
        [QueryKeys.FETCH_ESSENTIAL_SETTINGS],
      ];
      invalidateMultipleQueries(queryClient, keys);
      toaster.success(LL.settingsPage.messages.editSuccess());
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
      console.error(err);
    },
  });

  const zodSchema = useMemo(
    () =>
      z.object({
        main_logo_url: z.string(),
        nav_logo_url: z.string(),
        instance_name: z
          .string()
          .min(3, LL.form.error.minimumLength())
          .max(12, LL.form.error.maximumLength()),
        openid_enabled: z.boolean(),
        wireguard_enabled: z.boolean(),
        worker_enabled: z.boolean(),
        webhooks_enabled: z.boolean(),
        license: z.string().optional(),
      }),
    [LL.form.error],
  );

  const defaultValues = useMemo((): FormFields => {
    return {
      instance_name: settings?.instance_name ?? '',
      main_logo_url:
        settings?.main_logo_url === defaultSettings.main_logo_url
          ? ''
          : (settings?.main_logo_url ?? ''),
      nav_logo_url:
        settings?.nav_logo_url === defaultSettings.nav_logo_url
          ? ''
          : (settings?.nav_logo_url ?? ''),
      openid_enabled: settings?.openid_enabled ?? false,
      wireguard_enabled: settings?.wireguard_enabled ?? false,
      worker_enabled: settings?.worker_enabled ?? false,
      webhooks_enabled: settings?.webhooks_enabled ?? false,
      license: settings?.license ?? '',
    };
  }, [settings]);

  const { control, handleSubmit, setValue } = useForm<FormFields>({
    defaultValues,
    mode: 'all',
    resolver: zodResolver(zodSchema),
  });

  const onSubmit: SubmitHandler<FormFields> = (submitted) => {
    mutate(submitted);
  };

  if (!settings) return null;

  return (
    <section id="branding-settings">
      <header>
        <h2>{LL.settingsPage.tabs.global()}</h2>
        <div className="controls">
          <Button
            form="global-settings-form"
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
      </header>
      <form id="global-settings-form" onSubmit={handleSubmit(onSubmit)}>
        <div className="column-layout">
          <div className="left">
            <div>
              <div className="subsection-header-with-controls">
                <div className="helper-row">
                  <h3>{LL.settingsPage.instanceBranding.header()}</h3>
                  <Helper>
                    {parse(
                      LL.settingsPage.instanceBranding.helper({
                        documentationLink: externalLink.gitbook.base,
                      }),
                    )}
                  </Helper>
                </div>
                <Button
                  text={
                    breakpoint !== 'mobile'
                      ? LL.settingsPage.instanceBranding.form.controls.restoreDefault()
                      : undefined
                  }
                  size={ButtonSize.SMALL}
                  icon={<SvgIconX />}
                  styleVariant={ButtonStyleVariant.LINK}
                  loading={isLoading}
                  onClick={() => {
                    setValue('instance_name', defaultSettings.instance_name);
                    setValue('main_logo_url', '');
                    setValue('nav_logo_url', '');
                  }}
                />
              </div>
              <FormInput
                label={LL.settingsPage.instanceBranding.form.fields.instanceName.label()}
                controller={{ control, name: 'instance_name' }}
                placeholder={LL.settingsPage.instanceBranding.form.fields.instanceName.placeholder()}
                required
              />
              <FormInput
                labelExtras={
                  <Helper>
                    {LL.settingsPage.instanceBranding.form.fields.mainLogoUrl.helper()}
                  </Helper>
                }
                label={LL.settingsPage.instanceBranding.form.fields.mainLogoUrl.label()}
                controller={{ control, name: 'main_logo_url' }}
                placeholder={LL.settingsPage.instanceBranding.form.fields.mainLogoUrl.placeholder()}
                required
              />
              <FormInput
                labelExtras={
                  <Helper>
                    {LL.settingsPage.instanceBranding.form.fields.navLogoUrl.helper()}
                  </Helper>
                }
                label={LL.settingsPage.instanceBranding.form.fields.navLogoUrl.label()}
                controller={{ control, name: 'nav_logo_url' }}
                placeholder={LL.settingsPage.instanceBranding.form.fields.navLogoUrl.placeholder()}
                required
              />
            </div>
            <div>
              <div className="helper-row subsection-header">
                <h3>{LL.settingsPage.modulesVisibility.header()}</h3>
                <Helper>
                  {parse(
                    LL.settingsPage.modulesVisibility.helper({
                      documentationLink: externalLink.gitbook.base,
                    }),
                  )}
                </Helper>
              </div>
              <div className="checkbox-column">
                <FormCheckBox
                  disabled={isLoading}
                  label={LL.settingsPage.modulesVisibility.fields.openid_enabled.label()}
                  value={settings.openid_enabled}
                  controller={{
                    control,
                    name: 'openid_enabled',
                  }}
                  labelPlacement="right"
                />
                <FormCheckBox
                  label={LL.settingsPage.modulesVisibility.fields.wireguard_enabled.label()}
                  value={settings.wireguard_enabled}
                  disabled={isLoading}
                  controller={{
                    control,
                    name: 'wireguard_enabled',
                  }}
                  labelPlacement="right"
                />
                <FormCheckBox
                  label={LL.settingsPage.modulesVisibility.fields.worker_enabled.label()}
                  value={settings.worker_enabled}
                  disabled={isLoading}
                  controller={{
                    control,
                    name: 'worker_enabled',
                  }}
                  labelPlacement="right"
                />
                <FormCheckBox
                  label={LL.settingsPage.modulesVisibility.fields.webhooks_enabled.label()}
                  value={settings.webhooks_enabled}
                  disabled={isLoading}
                  controller={{
                    control,
                    name: 'webhooks_enabled',
                  }}
                  labelPlacement="right"
                />
              </div>
            </div>
          </div>
          <div className="right">
            <LicenseSettings control={control} />
          </div>
        </div>
      </form>
    </section>
  );
};
