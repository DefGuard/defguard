import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo } from 'react';
import { FormProvider, SubmitHandler, useForm } from 'react-hook-form';
import ReactMarkdown from 'react-markdown';
import { z } from 'zod';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../shared/components/svg/IconCheckmarkWhite';
import SvgIconX from '../../../../../shared/components/svg/IconX';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { MessageBox } from '../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { OpenIdProvider } from '../../../../../shared/types';
import { DirsyncSettings } from './DirectorySyncSettings';
import { OpenIdGeneralSettings } from './OpenIdGeneralSettings';
import { OpenIdProviderSettings } from './OpenIdProviderSettings';
import { SUPPORTED_SYNC_PROVIDERS } from './SupportedProviders';

export type UsernameHandling =
  | 'RemoveForbidden'
  | 'ReplaceForbidden'
  | 'PruneEmailDomain';
type FormFields = OpenIdProvider & {
  create_account: boolean;
  username_handling: UsernameHandling;
};

export const OpenIdSettingsForm = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.openIdSettings;
  const queryClient = useQueryClient();
  const enterpriseEnabled = useAppStore((s) => s.appInfo?.license_info.enterprise);
  const {
    settings: { testDirsync },
  } = useApi();

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
          username_handling: z.string(),
          okta_private_jwk: z.string(),
          okta_dirsync_client_id: z.string(),
          directory_sync_group_match: z.string(),
        })
        .superRefine((val, ctx) => {
          if (val.name === '') {
            ctx.addIssue({
              code: z.ZodIssueCode.custom,
              message: LL.form.error.required(),
              path: ['name'],
            });
          }

          if (val.directory_sync_enabled && val.base_url.includes('okta')) {
            if (val.okta_dirsync_client_id.length === 0) {
              ctx.addIssue({
                code: z.ZodIssueCode.custom,
                message: LL.form.error.required(),
                path: ['okta_dirsync_client_id'],
              });
            }
          }

          if (val.directory_sync_enabled && val.name === 'Google') {
            if (val.admin_email.length === 0) {
              ctx.addIssue({
                code: z.ZodIssueCode.custom,
                message: LL.form.error.required(),
                path: ['admin_email'],
              });
            }

            if (val.google_service_account_email.length === 0) {
              ctx.addIssue({
                code: z.ZodIssueCode.custom,
                message: LL.form.error.required(),
                path: ['google_service_account_email'],
              });
            }
          }
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
      okta_private_jwk: '',
      okta_dirsync_client_id: '',
      directory_sync_group_match: '',
      username_handling: 'RemoveForbidden',
    };

    if (openidData) {
      if (openidData.provider) {
        defaults = {
          ...defaults,
          ...openidData.provider,
        };

        if (Array.isArray(openidData.provider.directory_sync_group_match)) {
          defaults = {
            ...defaults,

            directory_sync_group_match:
              openidData.provider.directory_sync_group_match.length > 0
                ? openidData.provider.directory_sync_group_match.join(',')
                : '',
          };
        }
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

  const showDirsync = SUPPORTED_SYNC_PROVIDERS.includes(openidData?.provider?.name ?? '');

  return (
    <>
      <header>
        <h2>{localLL.heading()}</h2>
        <div className="controls">
          {showDirsync && (
            <Button
              onClick={() => {
                void testDirsync().then((res) => {
                  if (res.success) {
                    toaster.success(
                      localLL.form.directory_sync_settings.connectionTest.success(),
                    );
                  } else {
                    toaster.error(
                      `${localLL.form.directory_sync_settings.connectionTest.error()} ${res.message}`,
                    );
                  }
                });
              }}
              disabled={!enterpriseEnabled}
              text="Test OpenID connection"
              styleVariant={ButtonStyleVariant.LINK}
              size={ButtonSize.SMALL}
            ></Button>
          )}
          <Button
            text={localLL.form.delete()}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.CONFIRM}
            loading={isLoading}
            icon={<SvgIconX />}
            onClick={() => {
              handleDeleteProvider();
            }}
            disabled={!enterpriseEnabled}
          />
          <Button
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.SAVE}
            text={LL.common.controls.saveChanges()}
            type="submit"
            loading={isLoading}
            form="openid-form"
            icon={<IconCheckmarkWhite />}
            disabled={!enterpriseEnabled}
          />
        </div>
      </header>
      <FormProvider {...formControl}>
        <form
          id="openid-form"
          className="column-layout"
          onSubmit={handleSubmit(handleValidSubmit)}
        >
          <div className="left">
            <MessageBox type={MessageBoxType.INFO}>
              <ReactMarkdown>{localLL.form.documentation()}</ReactMarkdown>
            </MessageBox>
            <OpenIdGeneralSettings isLoading={isLoading} />
            <OpenIdProviderSettings isLoading={isLoading} />
          </div>
          <div className="right">
            <DirsyncSettings isLoading={isLoading} />
          </div>
        </form>
      </FormProvider>
    </>
  );
};
