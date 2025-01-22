import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo } from 'react';
import { FormProvider, SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../shared/components/svg/IconCheckmarkWhite';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { OpenIdProvider } from '../../../../../shared/types';
import { DirsyncSettings } from './DirectorySyncSettings';
import { OpenIdGeneralSettings } from './OpenIdGeneralSettings';
import { OpenIdSettingsForm } from './OpenIdProviderSettings';

type FormFields = OpenIdProvider & {
  create_account: boolean;
};

export const OpenIdSettingsRootForm = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.openIdSettings;
  const queryClient = useQueryClient();
  const enterpriseEnabled = useAppStore((s) => s.appInfo?.license_info.enterprise);

  const {
    settings: { fetchOpenIdProviders, addOpenIdProvider, deleteOpenIdProvider },
  } = useApi();

  const { isLoading, data: openidData } = useQuery({
    queryFn: fetchOpenIdProviders,
    queryKey: [QueryKeys.FETCH_OPENID_PROVIDERS],
    refetchOnMount: true,
    refetchOnWindowFocus: false,
    retry: false,
    enabled: enterpriseEnabled,
  });

  const toaster = useToaster();

  const { mutate } = useMutation({
    mutationFn: addOpenIdProvider,
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_OPENID_PROVIDERS],
      });
      toaster.success(LL.settingsPage.messages.editSuccess());
    },
    onError: (error) => {
      toaster.error(LL.messages.error());
      console.error(error);
    },
  });

  const { mutate: deleteProvider } = useMutation({
    mutationFn: deleteOpenIdProvider,
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_OPENID_PROVIDERS],
      });
      toaster.success(LL.settingsPage.messages.editSuccess());
    },
    onError: (error) => {
      toaster.error(LL.messages.error());
      console.error(error);
    },
  });

  const schema = useMemo(
    () =>
      z
        .object({
          name: z.string().optional(),
          base_url: z
            .string()
            .url(LL.form.error.invalid())
            .min(1, LL.form.error.required()),
          client_id: z.string().min(1, LL.form.error.required()),
          client_secret: z.string().min(1, LL.form.error.required()),
          display_name: z.string(),
          admin_email: z.string(),
          google_service_account_email: z.string(),
          google_service_account_key: z.string(),
          directory_sync_enabled: z.boolean(),
          directory_sync_interval: z.number().min(60, LL.form.error.invalid()),
          directory_sync_user_behavior: z.string(),
          directory_sync_admin_behavior: z.string(),
          directory_sync_target: z.string(),
          create_account: z.boolean(),
        })
        .superRefine((val, ctx) => {
          if (val.name === '') {
            ctx.addIssue({
              code: z.ZodIssueCode.custom,
              message: LL.form.error.required(),
              path: ['name'],
            });
          }

          // if (val.directory_sync_enabled && val.name !== 'Google') {
          //   if (val.admin_email.length === 0) {
          //     ctx.addIssue({
          //       code: z.ZodIssueCode.custom,
          //       message: LL.form.error.required(),
          //       path: ['admin_email'],
          //     });
          //   }

          //   if (val.google_service_account_email.length === 0) {
          //     ctx.addIssue({
          //       code: z.ZodIssueCode.custom,
          //       message: LL.form.error.required(),
          //       path: ['google_service_account_email'],
          //     });
          //   }
          // }
        }),
    [LL.form.error],
  );

  const defaultValues = useMemo((): FormFields => {
    let defaults: FormFields = {
      id: 0,
      name: '',
      base_url: '',
      client_id: '',
      client_secret: '',
      display_name: '',
      admin_email: '',
      google_service_account_email: '',
      google_service_account_key: '',
      directory_sync_enabled: false,
      directory_sync_interval: 600,
      directory_sync_user_behavior: 'keep',
      directory_sync_admin_behavior: 'keep',
      directory_sync_target: 'all',
      create_account: false,
    };

    if (openidData) {
      if (openidData.provider) {
        defaults = {
          ...defaults,
          ...openidData.provider,
        };
      }

      defaults = {
        ...defaults,
        ...openidData.settings,
      };
    }

    return defaults;
  }, [openidData]);

  const formControl = useForm<FormFields>({
    resolver: zodResolver(schema),
    defaultValues,
    mode: 'all',
  });

  const { handleSubmit, reset } = formControl;

  // Make sure the form data is fresh
  useEffect(() => {
    reset(defaultValues);
  }, [defaultValues, reset]);

  const handleValidSubmit: SubmitHandler<FormFields> = (data) => {
    mutate(data);
  };

  const handleDeleteProvider = useCallback(() => {
    if (openidData?.provider) {
      deleteProvider(openidData?.provider.name);
    }
  }, [openidData, deleteProvider]);

  return (
    <FormProvider {...formControl}>
      <form id="root-form" onSubmit={handleSubmit(handleValidSubmit)}>
        <div className="controls">
          <Button
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.SAVE}
            text={LL.common.controls.saveChanges()}
            type="submit"
            loading={isLoading}
            form="root-form"
            icon={<IconCheckmarkWhite />}
            disabled={!enterpriseEnabled}
          />
          <Button
            text={localLL.form.delete()}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.CONFIRM}
            loading={isLoading}
            onClick={() => {
              handleDeleteProvider();
            }}
            disabled={!enterpriseEnabled}
          />
        </div>
        <div className="left">
          <OpenIdSettingsForm isLoading={isLoading} />
        </div>
        <div className="right">
          <OpenIdGeneralSettings isLoading={isLoading} />
          <DirsyncSettings isLoading={isLoading} />
        </div>
      </form>
    </FormProvider>
  );
};
