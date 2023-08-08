import './styles.scss';

import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import parse from 'html-react-parser';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useBreakpoint } from 'use-breakpoint';
import * as yup from 'yup';

import { useI18nContext } from '../../../i18n/i18n-react';
import { IconCheckmarkWhite } from '../../../shared/components/svg';
import { deviceBreakpoints } from '../../../shared/constants';
import { useAppStore } from '../../../shared/hooks/store/useAppStore';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../shared/mutations';
import { patternValidEmail } from '../../../shared/patterns';
import { QueryKeys } from '../../../shared/queries';
import { Settings } from '../../../shared/types';
import { validateIpOrDomain } from '../../../shared/validators';
import { TestForm } from './TestForm';
import { FormInput } from '../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { Button } from '../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../../shared/defguard-ui/components/Layout/Card/Card';
import { Helper } from '../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { SelectOption } from '../../../shared/defguard-ui/components/Layout/Select/types';

interface Inputs extends Omit<Settings, 'smtp_encryption'> {
  smtp_encryption: SelectOption<string>;
}

export const SmtpCard = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    settings: { editSettings },
  } = useApi();

  const [settings] = useAppStore((state) => [state.settings, state.setAppStore]);

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
          smtp_encryption: yup.object().required(),
          smtp_user: yup.string(),
          smtp_password: yup.string(),
          smtp_sender: yup.string().matches(patternValidEmail, LL.form.error.invalid()),
        })
        .required(),
    [LL.form.error],
  );

  const encryptionOptions = [
    {
      key: 1,
      value: 'None',
      label: LL.settingsPage.smtp.form.fields.encryption.none(),
    },
    {
      key: 2,
      value: 'StartTls',
      label: 'Start TLS',
    },
    {
      key: 3,
      value: 'ImplicitTls',
      label: 'Implicit TLS',
    },
  ];

  const defaultEncryption =
    encryptionOptions.filter((option) => option.value === settings?.smtp_encryption)[0] ||
    encryptionOptions[1];
  const { control, handleSubmit } = useForm<Inputs>({
    defaultValues: useMemo(() => {
      return {
        smtp_server: settings?.smtp_server,
        smtp_port: settings?.smtp_port,
        smtp_encryption: defaultEncryption,
        smtp_user: settings?.smtp_user,
        smtp_password: settings?.smtp_password,
        smtp_sender: settings?.smtp_sender,
      };
    }, [
      settings?.smtp_server,
      settings?.smtp_port,
      settings?.smtp_user,
      settings?.smtp_password,
      settings?.smtp_sender,
      defaultEncryption,
    ]),
    resolver: yupResolver(formSchema),
    mode: 'all',
  });

  if (!settings) return null;

  const onSubmit: SubmitHandler<Inputs> = (data) => {
    mutate({ ...settings, ...data, smtp_encryption: data.smtp_encryption.value });
  };

  return (
    <section className="smtp">
      <header>
        <h2>{LL.settingsPage.smtp.header()}</h2>
        <Helper>{parse(LL.settingsPage.smtp.helper())}</Helper>
      </header>
      <Card>
        <header>
          <h3>{LL.settingsPage.smtp.form.title()}</h3>
          <div className="controls">
            <Button
              form="smtp-form"
              text={
                breakpoint !== 'mobile'
                  ? LL.settingsPage.smtp.form.controls.submit()
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
        <form id="smtp-form" onSubmit={handleSubmit(onSubmit)}>
          <FormInput
            outerLabel={LL.settingsPage.smtp.form.fields.server.label()}
            controller={{ control, name: 'smtp_server' }}
            placeholder={LL.settingsPage.smtp.form.fields.server.placeholder()}
            required
          />
          <FormInput
            outerLabel={LL.settingsPage.smtp.form.fields.port.label()}
            controller={{ control, name: 'smtp_port' }}
            placeholder={LL.settingsPage.smtp.form.fields.port.placeholder()}
            required
          />
          <FormInput
            outerLabel={LL.settingsPage.smtp.form.fields.user.label()}
            controller={{ control, name: 'smtp_user' }}
            placeholder={LL.settingsPage.smtp.form.fields.user.placeholder()}
            required
          />
          <FormInput
            outerLabel={LL.settingsPage.smtp.form.fields.password.label()}
            controller={{ control, name: 'smtp_password' }}
            placeholder={LL.settingsPage.smtp.form.fields.password.placeholder()}
            required
          />
          <FormInput
            outerLabel={LL.settingsPage.smtp.form.fields.sender.label()}
            controller={{ control, name: 'smtp_sender' }}
            placeholder={LL.settingsPage.smtp.form.fields.sender.placeholder()}
            required
          />
          <Helper>{parse(LL.settingsPage.smtp.form.fields.sender.helper())}</Helper>
          <FormSelect
            data-testid="groups-select"
            styleVariant={SelectStyleVariant.WHITE}
            options={encryptionOptions}
            controller={{ control, name: 'smtp_encryption' }}
            outerLabel={LL.settingsPage.smtp.form.fields.encryption.label()}
          />
        </form>
        <TestForm />
      </Card>
    </section>
  );
};
