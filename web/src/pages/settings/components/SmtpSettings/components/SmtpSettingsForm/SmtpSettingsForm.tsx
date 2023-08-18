import './style.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import parse from 'html-react-parser';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import * as yup from 'yup';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../../shared/components/svg/IconCheckmarkWhite';
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
import { MutationKeys } from '../../../../../../shared/mutations';
import { patternValidEmail } from '../../../../../../shared/patterns';
import { QueryKeys } from '../../../../../../shared/queries';
import { validateIpOrDomain } from '../../../../../../shared/validators';

type FormFields = {
  smtp_server: string;
  smtp_port: string;
  smtp_user: string;
  smtp_password: string;
  smtp_sender: string;
};

export const SmtpSettingsForm = () => {
  const { LL } = useI18nContext();
  const settings = useAppStore((state) => state.settings);

  const toaster = useToaster();

  const {
    settings: { editSettings },
  } = useApi();

  const queryClient = useQueryClient();

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

  const formSchema = useMemo(
    () =>
      yup
        .object()
        .shape({
          smtp_server: yup
            .string()
            .test(LL.form.error.endpoint(), (val: string | undefined) =>
              !val ? true : validateIpOrDomain(val),
            ),
          smtp_port: yup
            .number()
            .max(65535, LL.form.error.portMax())
            .typeError(LL.form.error.validPort()),
          smtp_user: yup.string(),
          smtp_password: yup.string(),
          smtp_sender: yup.string().matches(patternValidEmail, LL.form.error.invalid()),
        })
        .required(),
    [LL.form.error],
  );

  const defaultValues = useMemo(() => {
    const res: FormFields = {
      smtp_server: settings?.smtp_server ?? '',
      smtp_port: '',
      smtp_password: settings?.smtp_password ?? '',
      smtp_sender: settings?.smtp_sender ?? '',
      smtp_user: settings?.smtp_user ?? '',
    };
    if (settings && settings.smtp_port) {
      res.smtp_port = String(settings.smtp_port);
    }
    return res;
  }, [settings]);

  const { control, handleSubmit } = useForm<FormFields>({
    defaultValues,
    resolver: yupResolver(formSchema),
    mode: 'all',
  });

  const onSubmit: SubmitHandler<FormFields> = (data) => {
    if (settings) {
      mutate({
        ...settings,
        ...data,
        smtp_port: data.smtp_port.length > 0 ? parseInt(data.smtp_port) : undefined,
      });
    }
  };

  if (!settings) return null;

  return (
    <section id="smtp-settings">
      <header>
        <h2>{LL.settingsPage.smtp.form.title()}</h2>
        <Helper>{parse(LL.settingsPage.smtp.helper())}</Helper>
      </header>
      <Card shaded bordered>
        <div className="controls">
          <Button
            form="smtp-form"
            text={LL.settingsPage.smtp.form.controls.submit()}
            icon={<IconCheckmarkWhite />}
            size={ButtonSize.SMALL}
            styleVariant={ButtonStyleVariant.SAVE}
            loading={isLoading}
            type="submit"
          />
        </div>
        <form id="smtp-form" onSubmit={handleSubmit(onSubmit)}>
          <FormInput
            label={LL.settingsPage.smtp.form.fields.server.label()}
            controller={{ control, name: 'smtp_server' }}
            placeholder={LL.settingsPage.smtp.form.fields.server.placeholder()}
            required
          />
          <FormInput
            label={LL.settingsPage.smtp.form.fields.port.label()}
            controller={{ control, name: 'smtp_port' }}
            placeholder={LL.settingsPage.smtp.form.fields.port.placeholder()}
            required
          />
          <FormInput
            label={LL.settingsPage.smtp.form.fields.user.label()}
            controller={{ control, name: 'smtp_user' }}
            placeholder={LL.settingsPage.smtp.form.fields.user.placeholder()}
            required
          />
          <FormInput
            label={LL.settingsPage.smtp.form.fields.password.label()}
            controller={{ control, name: 'smtp_password' }}
            placeholder={LL.settingsPage.smtp.form.fields.password.placeholder()}
            required
          />
          <FormInput
            labelExtras={
              <Helper>{parse(LL.settingsPage.smtp.form.fields.sender.helper())}</Helper>
            }
            label={LL.settingsPage.smtp.form.fields.sender.label()}
            controller={{ control, name: 'smtp_sender' }}
            placeholder={LL.settingsPage.smtp.form.fields.sender.placeholder()}
            required
          />
        </form>
      </Card>
    </section>
  );
};
