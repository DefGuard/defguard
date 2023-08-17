import './styles.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import parse from 'html-react-parser';
import { useEffect, useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useBreakpoint } from 'use-breakpoint';
import * as yup from 'yup';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../../shared/components/svg/IconCheckmarkWhite';
import { deviceBreakpoints } from '../../../../../../shared/constants';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { Helper } from '../../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { useAppStore } from '../../../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { externalLink } from '../../../../../../shared/links';
import { MutationKeys } from '../../../../../../shared/mutations';
import { QueryKeys } from '../../../../../../shared/queries';
import { Settings } from '../../../../../../shared/types';

type FormFields = {
  instance_name: string;
  main_logo_url: string;
  nav_logo_url: string;
};

const defaultSettings: FormFields = {
  instance_name: 'Defguard',
  main_logo_url: '/svg/logo-defguard-white.svg',
  nav_logo_url: '/svg/defguard-nav-logo.svg',
};

export const BrandingSettings = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    settings: { editSettings, setDefaultBranding },
  } = useApi();

  const [settings, setAppStore] = useAppStore((state) => [
    state.settings,
    state.setAppStore,
  ]);

  const queryClient = useQueryClient();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);

  const { mutate, isLoading } = useMutation([MutationKeys.EDIT_SETTINGS], editSettings, {
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
      toaster.success(LL.settingsPage.messages.editSuccess());
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
      console.error(err);
    },
  });

  const { mutate: setDefaultBrandingMutation } = useMutation(
    [MutationKeys.EDIT_SETTINGS],
    setDefaultBranding,
    {
      onSuccess: (settings) => {
        setAppStore({ settings });
        toaster.success(LL.settingsPage.messages.editSuccess());
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    },
  );

  const formSchema = useMemo(
    () =>
      yup
        .object()
        .shape({
          main_logo_url: yup.string(),
          nav_logo_url: yup.string(),
          instance_name: yup
            .string()
            .min(3, LL.form.error.minimumLength())
            .max(12, LL.form.error.maximumLength())
            .required(LL.form.error.required()),
        })
        .required(),
    [LL.form.error],
  );

  const defaultValues = useMemo((): FormFields => {
    return {
      instance_name: settings?.instance_name ?? '',
      main_logo_url:
        settings?.main_logo_url === defaultSettings.main_logo_url
          ? ''
          : settings?.main_logo_url ?? '',
      nav_logo_url:
        settings?.nav_logo_url === defaultSettings.nav_logo_url
          ? ''
          : settings?.nav_logo_url ?? '',
    };
  }, [settings?.instance_name, settings?.main_logo_url, settings?.nav_logo_url]);

  const { control, handleSubmit, reset } = useForm<Settings>({
    defaultValues,
    mode: 'all',
    resolver: yupResolver(formSchema),
  });

  useEffect(() => {
    reset();
  }, [reset, defaultValues]);

  const onSubmit: SubmitHandler<FormFields> = (data) => {
    if (settings) {
      const res = { ...settings };
      res.instance_name = data.instance_name;
      if (data.main_logo_url != '') {
        res.main_logo_url = data.main_logo_url;
      }
      if (data.nav_logo_url != '') {
        res.nav_logo_url = data.nav_logo_url;
      }
      mutate(res);
    }
  };

  return (
    <section id="branding-settings">
      <header>
        <h2>{LL.settingsPage.instanceBranding.header()}</h2>
        <Helper>
          {parse(
            LL.settingsPage.instanceBranding.helper({
              documentationLink: externalLink.gitbook.base,
            }),
          )}
        </Helper>
      </header>
      <Card shaded bordered hideMobile>
        <div className="controls">
          <h3>{LL.settingsPage.instanceBranding.form.title()}</h3>
          <Button
            text={
              breakpoint !== 'mobile'
                ? LL.settingsPage.instanceBranding.form.controls.restoreDefault()
                : undefined
            }
            size={ButtonSize.SMALL}
            icon={<IconCheckmarkWhite />}
            styleVariant={ButtonStyleVariant.PRIMARY}
            loading={isLoading}
            onClick={() => setDefaultBrandingMutation('1')}
          />
          <Button
            form="branding-form"
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
        <form id="branding-form" onSubmit={handleSubmit(onSubmit)}>
          <FormInput
            label={LL.settingsPage.instanceBranding.form.fields.instanceName.label()}
            controller={{ control, name: 'instance_name' }}
            placeholder={LL.settingsPage.instanceBranding.form.fields.instanceName.placeholder()}
            required
          />
          <FormInput
            labelExtras={
              <Helper>
                {parse(LL.settingsPage.instanceBranding.form.fields.mainLogoUrl.helper())}
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
                {parse(LL.settingsPage.instanceBranding.form.fields.navLogoUrl.helper())}
              </Helper>
            }
            label={LL.settingsPage.instanceBranding.form.fields.navLogoUrl.label()}
            controller={{ control, name: 'nav_logo_url' }}
            placeholder={LL.settingsPage.instanceBranding.form.fields.navLogoUrl.placeholder()}
            required
          />
        </form>
      </Card>
    </section>
  );
};
