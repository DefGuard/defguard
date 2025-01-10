import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import parse from 'html-react-parser';
import { useCallback, useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../../shared/components/svg/IconCheckmarkWhite';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Helper } from '../../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import {
  SelectOption,
  SelectSelectedValue,
} from '../../../../../../shared/defguard-ui/components/Layout/Select/types';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { patternValidEmail } from '../../../../../../shared/patterns';
import { QueryKeys } from '../../../../../../shared/queries';
import { SettingsSMTP } from '../../../../../../shared/types';
import { invalidateMultipleQueries } from '../../../../../../shared/utils/invalidateMultipleQueries';
import { validateIpOrDomain } from '../../../../../../shared/validators';
import { useSettingsPage } from '../../../../hooks/useSettingsPage';

type FormFields = {
  smtp_server: string;
  smtp_port: number;
  smtp_user: string;
  smtp_password: string;
  smtp_sender: string;
  smtp_encryption: string;
};

export const SmtpSettingsForm = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.smtp;

  const settings = useSettingsPage((state) => state.settings);

  const toaster = useToaster();

  const {
    settings: { patchSettings },
  } = useApi();

  const queryClient = useQueryClient();

  const { mutate, isPending: isLoading } = useMutation({
    mutationFn: patchSettings,
    onSuccess: () => {
      invalidateMultipleQueries(queryClient, [
        [QueryKeys.FETCH_APP_INFO],
        [QueryKeys.FETCH_SETTINGS],
      ]);
      toaster.success(LL.settingsPage.messages.editSuccess());
    },
    onError: (err) => {
      toaster.error(LL.messages.error());
      console.error(err);
    },
  });

  const encryptionOptions = useMemo(
    (): SelectOption<string>[] => [
      {
        key: 1,
        value: 'StartTls',
        label: 'Start TLS',
      },
      {
        key: 2,
        value: 'None',
        label: 'None',
      },
      {
        key: 3,
        value: 'ImplicitTls',
        label: 'Implicit TLS',
      },
    ],
    [],
  );

  const renderSelectedEncryption = useCallback(
    (selected: string): SelectSelectedValue => {
      const option = encryptionOptions.find((o) => o.value === selected);
      if (!option) throw Error("Selected value doesn't exist");
      return {
        key: option.key,
        displayValue: option.label,
      };
    },
    [encryptionOptions],
  );

  const zodSchema = useMemo(
    () =>
      z.object({
        smtp_server: z
          .string()
          .min(1, LL.form.error.required())
          .refine(
            (val) => (!val ? true : validateIpOrDomain(val, false, true)),
            LL.form.error.endpoint(),
          ),
        smtp_port: z
          .number({
            invalid_type_error: LL.form.error.required(),
          })
          .max(65535, LL.form.error.portMax()),
        smtp_password: z.string(),
        smtp_user: z.string(),
        smtp_sender: z
          .string()
          .min(1, LL.form.error.required())
          .regex(patternValidEmail, LL.form.error.invalid()),
        smtp_encryption: z.string().min(1, LL.form.error.required()),
      }),
    [LL.form],
  );

  const defaultValues = useMemo(() => {
    const res: FormFields = {
      smtp_server: settings?.smtp_server ?? '',
      smtp_port: settings?.smtp_port ?? 587,
      smtp_password: settings?.smtp_password ?? '',
      smtp_sender: settings?.smtp_sender ?? '',
      smtp_user: settings?.smtp_user ?? '',
      smtp_encryption: settings?.smtp_encryption ?? encryptionOptions[1].value,
    };
    return res;
  }, [settings, encryptionOptions]);

  const emptyValues: SettingsSMTP = useMemo(
    () => ({
      smtp_server: '',
      smtp_port: 587,
      smtp_password: '',
      smtp_sender: '',
      smtp_user: '',
      smtp_encryption: encryptionOptions[1].value,
    }),
    [encryptionOptions],
  );

  const { control, reset, handleSubmit } = useForm<FormFields>({
    defaultValues,
    mode: 'all',
    resolver: zodResolver(zodSchema),
  });

  const onSubmit: SubmitHandler<FormFields> = (data) => {
    mutate(data);
  };

  const handleDeleteSubmit = useCallback(() => {
    mutate(emptyValues);
    reset(emptyValues);
  }, [mutate, emptyValues, reset]);

  if (!settings) return null;

  return (
    <section id="smtp-settings">
      <header>
        <h2>{localLL.form.title()}</h2>
        <Helper>{parse(localLL.helper())}</Helper>
        <div className="controls">
          <Button
            form="smtp-form"
            text={localLL.form.controls.submit()}
            icon={<IconCheckmarkWhite />}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.SAVE}
            loading={isLoading}
            type="submit"
          />
          <Button
            text={localLL.delete()}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.CONFIRM}
            loading={isLoading}
            onClick={() => {
              handleDeleteSubmit();
            }}
          />
        </div>
      </header>
      <form id="smtp-form" onSubmit={handleSubmit(onSubmit)}>
        <FormInput
          label={localLL.form.fields.server.label()}
          controller={{ control, name: 'smtp_server' }}
          placeholder={localLL.form.fields.server.placeholder()}
          required
        />
        <FormInput
          label={localLL.form.fields.port.label()}
          controller={{ control, name: 'smtp_port' }}
          placeholder={localLL.form.fields.port.placeholder()}
          type="number"
          required
        />
        <FormInput
          label={localLL.form.fields.user.label()}
          controller={{ control, name: 'smtp_user' }}
          placeholder={localLL.form.fields.user.placeholder()}
        />
        <FormInput
          label={localLL.form.fields.password.label()}
          controller={{ control, name: 'smtp_password' }}
          placeholder={localLL.form.fields.password.placeholder()}
          type="password"
        />
        <FormInput
          labelExtras={<Helper>{parse(localLL.form.fields.sender.helper())}</Helper>}
          label={localLL.form.fields.sender.label()}
          controller={{ control, name: 'smtp_sender' }}
          placeholder={localLL.form.fields.sender.placeholder()}
          required
        />
        <FormSelect
          data-testid="smtp-encryption-select"
          label={localLL.form.fields.encryption.label()}
          renderSelected={renderSelectedEncryption}
          options={encryptionOptions}
          controller={{ control, name: 'smtp_encryption' }}
        />
      </form>
    </section>
  );
};
