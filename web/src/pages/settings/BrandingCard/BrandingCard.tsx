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

export const BrandingCard = () => {
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
        toaster.success('Settings changed.');
      },
      onError: (err) => {
        toaster.error('Error occured!', 'Please contact administrator');
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
        toaster.success('Settings changed.');
      },
      onError: (err) => {
        toaster.error('Error occured!', 'Please contact administrator');
        console.error(err);
      },
    }
  );

  const formSchema = useMemo(
    () =>
      yup
        .object()
        .shape({
          main_logo_url: yup.string().required('Url is required.'),
          nav_logo_url: yup.string().required('Url is required.'),
          instance_name: yup
            .string()
            .min(3, 'Should be at least 4 characters long.')
            .max(12, 'Maximum length exceeded.')
            .required('Name is required.'),
        })
        .required(),
    []
  );
  const { control, handleSubmit, reset } = useForm<Settings>({
    defaultValues: {
      instance_name: settings?.instance_name,
      main_logo_url:
        settings?.main_logo_url === defaultSettings.main_logo_url
          ? ''
          : settings?.main_logo_url,
      nav_logo_url:
        settings?.nav_logo_url === defaultSettings.nav_logo_url
          ? ''
          : settings?.nav_logo_url,
    },
    resolver: yupResolver(formSchema),
    mode: 'all',
  });

  if (!settings) return null;

  const onSubmit: SubmitHandler<Settings> = (data) => {
    settings.instance_name = data.instance_name;
    settings.main_logo_url = data.main_logo_url;
    settings.nav_logo_url = data.nav_logo_url;
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
        <h2>Instance Branding</h2>
        <Helper>
          <p>
            Here you can add url of your logo and name for your defguard
            instance it will be displayed instead of defguard.
          </p>{' '}
          <a href="defguard.gitbook.io" target="_blank">
            Read more in documentation.
          </a>
        </Helper>
      </header>
      <Card>
        <header>
          <h3>Name & Logo:</h3>
          <div className="controls">
            <Button
              text={breakpoint !== 'mobile' ? 'Restore default' : undefined}
              size={ButtonSize.SMALL}
              icon={<IconCheckmarkWhite />}
              styleVariant={ButtonStyleVariant.PRIMARY}
              loading={isLoading}
              disabled={disableRestoreDefault()}
              onClick={() => setDefaultBrandingMutation('1')}
            />
            <Button
              form="branding-form"
              text={breakpoint !== 'mobile' ? 'Save changes' : undefined}
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
            outerLabel="Instance name"
            controller={{ control, name: 'instance_name' }}
            placeholder="Example"
            required
          />
          <FormInput
            outerLabel="Login logo url"
            controller={{ control, name: 'main_logo_url' }}
            placeholder="Default image"
            required
          />

          <FormInput
            outerLabel="Nav Logo url"
            controller={{ control, name: 'nav_logo_url' }}
            placeholder="Default image"
            required
          />
        </form>
      </Card>
    </section>
  );
};
