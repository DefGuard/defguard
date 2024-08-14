import './styles.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import { useEffect, useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useBreakpoint } from 'use-breakpoint';
import { z } from 'zod';

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
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../../shared/queries';
import { Settings } from '../../../../../../shared/types';
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

  const queryClient = useQueryClient();
  const { breakpoint } = useBreakpoint(deviceBreakpoints);

  const { mutate, isLoading } = useMutation(patchSettings, {
    onSuccess: () => {
      toaster.success(LL.settingsPage.messages.editSuccess());
      queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
      queryClient.invalidateQueries([QueryKeys.FETCH_ENTERPRISE_STATUS]);
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

  const { control, handleSubmit, reset } = useForm<Settings>({
    defaultValues,
    mode: 'all',
    resolver: zodResolver(zodSchema),
  });

  useEffect(() => {
    reset();
  }, [reset, defaultValues]);

  const onSubmit: SubmitHandler<FormFields> = (submitted) => {
    mutate(submitted);
  };

  return (
    <section id="license-settings">
      <header>
        <h2>{LL.settingsPage.license.header()}</h2>
      </header>
      <Card shaded bordered>
        <div className="controls">
          <h3>{LL.settingsPage.license.form.title()}:</h3>
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
        <form id="license-form" onSubmit={handleSubmit(onSubmit)}>
          <FormInput
            label={LL.settingsPage.license.form.fields.key.label()}
            controller={{ control, name: 'license' }}
            placeholder={LL.settingsPage.license.form.fields.key.placeholder()}
          />
        </form>
      </Card>
    </section>
  );
};
