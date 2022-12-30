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

  const settings = useAppStore((state) => state.settings);

  const queryClient = useQueryClient();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);

  const { mutate, isLoading } = useMutation(
    [MutationKeys.EDIT_SETTINGS],
    editSettings,
    {
      onSuccess: () => {
        queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
        toaster.success('Settings changed.');
      },
      onError: () => {
        toaster.error('Error occured!', 'Please contact administrator');
      },
    }
  );
  const { mutate: setDefaultBrandingMutation } = useMutation(
    [MutationKeys.EDIT_SETTINGS],
    setDefaultBranding,
    {
      onSuccess: () => {
        queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
        toaster.success('Settings changed.');
      },
      onError: () => {
        toaster.error('Error occured!', 'Please contact administrator');
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
            .required(),
        })
        .required(),
    []
  );
  const { control, handleSubmit } = useForm<Settings>({
    defaultValues: settings,
    resolver: yupResolver(formSchema),
    mode: 'all',
  });

  console.log(settings);

  if (!settings) return null;

  const onSubmit: SubmitHandler<Settings> = (data) => {
    mutate(data);
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
            placeholder="https://example.com/logo.jpg"
            required
          />

          <FormInput
            outerLabel="Nav Logo url"
            controller={{ control, name: 'nav_logo_url' }}
            placeholder="https://example.com/logo.jpg"
            required
          />
        </form>
      </Card>
    </section>
  );
};
