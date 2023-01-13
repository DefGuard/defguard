import './styles.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import useBreakpoint from 'use-breakpoint';
import * as yup from 'yup';

import { FormInput } from '../../../shared/components/Form/FormInput/FormInput';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import { Card } from '../../../shared/components/layout/Card/Card';
import { Helper } from '../../../shared/components/layout/Helper/Helper';
import { IconCheckmarkWhite } from '../../../shared/components/svg';
import { deviceBreakpoints } from '../../../shared/constants';
import { useAppStore } from '../../../shared/hooks/store/useAppStore';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../shared/mutations';
import { QueryKeys } from '../../../shared/queries';
import { Settings } from '../../../shared/types';
import { useI18nContext } from '../../../i18n/i18n-react';
import parse from 'html-react-parser';
import MessageBox from '../../../shared/components/layout/MessageBox/MessageBox';

export const BrandingCard = () => {
  const { LL, locale } = useI18nContext();
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

  const defaultSettings = {
    instance_name: 'Defguard',
    main_logo_url: '/svg/logo-defguard-white.svg',
    nav_logo_url: '/svg/defguard-nav-logo.svg',
  };

  const { mutate, isLoading } = useMutation(
    [MutationKeys.EDIT_SETTINGS],
    editSettings,
    {
      onSuccess: () => {
        queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
        toaster.success(LL.settingsPage.messages.editSuccess());
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    }
  );
  const { mutate: setDefaultBrandingMutation } = useMutation(
    [MutationKeys.EDIT_SETTINGS],
    setDefaultBranding,
    {
      onSuccess: (settings) => {
        setAppStore({ settings });
        reset();
        queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
        toaster.success(LL.settingsPage.messages.editSuccess());
      },
      onError: (err) => {
        toaster.error(LL.messages.error());
        console.error(err);
      },
    }
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
    [locale]
  );
  const { control, handleSubmit, reset } = useForm<Settings>({
    defaultValues: useMemo(() => {
      return {
        instance_name: settings?.instance_name,
        main_logo_url:
          settings?.main_logo_url === defaultSettings.main_logo_url
            ? ''
            : settings?.main_logo_url,
        nav_logo_url:
          settings?.nav_logo_url === defaultSettings.nav_logo_url
            ? ''
            : settings?.nav_logo_url,
      };
    }, [settings]),
    resolver: yupResolver(formSchema),
    mode: 'all',
  });

  if (!settings) return null;

  const onSubmit: SubmitHandler<Settings> = (data) => {
    settings.instance_name = data.instance_name;
    if (data.main_logo_url != '') {
      settings.main_logo_url = data.main_logo_url;
    }
    if (data.nav_logo_url != '') {
      settings.nav_logo_url = data.nav_logo_url;
    }
    mutate(settings);
  };

  const disableRestoreDefault = () => {
    if (
      settings.instance_name === defaultSettings.instance_name &&
      settings.nav_logo_url === defaultSettings.nav_logo_url &&
      settings.main_logo_url === defaultSettings.main_logo_url
    ) {
      return true;
    } else {
      return false;
    }
  };

  return (
    <section className="branding">
      <header>
        <h2>{LL.settingsPage.instanceBranding.header()}</h2>
        <Helper>{parse(LL.settingsPage.instanceBranding.helper())}</Helper>
      </header>
      <Card>
        <header>
          <h3>{LL.settingsPage.instanceBranding.form.title()}</h3>
          <div className="controls">
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
              disabled={disableRestoreDefault()}
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
              styleVariant={ButtonStyleVariant.CONFIRM_SUCCESS}
              loading={isLoading}
              type="submit"
            />
          </div>
        </header>
        <form id="branding-form" onSubmit={handleSubmit(onSubmit)}>
          <FormInput
            outerLabel={LL.settingsPage.instanceBranding.form.fields.instanceName.label()}
            controller={{ control, name: 'instance_name' }}
            placeholder={LL.settingsPage.instanceBranding.form.fields.instanceName.placeholder()}
            required
          />
          <Helper>
              {parse(LL.settingsPage.instanceBranding.form.fields.mainLogoUrl.helper())}
          </Helper>
          <FormInput
            outerLabel={LL.settingsPage.instanceBranding.form.fields.mainLogoUrl.label()}
            controller={{ control, name: 'main_logo_url' }}
            placeholder={LL.settingsPage.instanceBranding.form.fields.mainLogoUrl.placeholder()}
            required
          />
          <Helper>
            <p>
              {parse(LL.settingsPage.instanceBranding.form.fields.navLogoUrl.helper())}
            </p>
          </Helper>
          <FormInput
            outerLabel={LL.settingsPage.instanceBranding.form.fields.navLogoUrl.label()}
            controller={{ control, name: 'nav_logo_url' }}
            placeholder={LL.settingsPage.instanceBranding.form.fields.navLogoUrl.placeholder()}
            required
          />
        </form>
      </Card>
    </section>
  );
};
