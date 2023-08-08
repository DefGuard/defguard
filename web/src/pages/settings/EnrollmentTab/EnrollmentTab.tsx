import { yupResolver } from '@hookform/resolvers/yup';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import parse from 'html-react-parser';
import { useEffect, useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useBreakpoint } from 'use-breakpoint';
import * as yup from 'yup';

import { useI18nContext } from '../../../i18n/i18n-react';
import { FormCheckBox } from '../../../shared/components/Form/FormCheckBox/FormCheckBox';
import { FormSelect } from '../../../shared/components/Form/FormSelect/FormSelect';
import { Button } from '../../../shared/components/layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/types';
import { Card } from '../../../shared/components/layout/Card/Card';
import { Helper } from '../../../shared/components/layout/Helper/Helper';
import MessageBox, {
  MessageBoxType,
} from '../../../shared/components/layout/MessageBox/MessageBox';
import {
  SelectOption,
  SelectStyleVariant,
} from '../../../shared/components/layout/Select/Select';
import { IconCheckmarkWhite } from '../../../shared/components/svg/index';
import { deviceBreakpoints } from '../../../shared/constants';
import { useAppStore } from '../../../shared/hooks/store/useAppStore';
import useApi from '../../../shared/hooks/useApi';
import { useToaster } from '../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../shared/mutations';
import { QueryKeys } from '../../../shared/queries';
import { Settings } from '../../../shared/types';

interface Inputs extends Omit<Settings, 'enrollment_vpn_step_optional'> {
  enrollment_vpn_step_optional: SelectOption<boolean>;
}

export const EnrollmentTab = () => {
  const [welcomeMessage, setWelcomeMessage] = useState('');
  const [welcomeEmail, setWelcomeEmail] = useState('');
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

  const formSchema = yup
    .object()
    .shape({
      enrollment_vpn_step_optional: yup.object(),
      enrollment_welcome_message: yup.string(),
      enrollment_welcome_email: yup.string(),
      enrollment_use_welcome_message_as_email: yup.boolean(),
    })
    .required();

  const vpnOptionalityOptions = [
    {
      key: 1,
      value: true,
      label: 'Optional',
    },
    {
      key: 2,
      value: false,
      label: 'Mandatory',
    },
  ];

  const defaultVpnOptionality =
    vpnOptionalityOptions.filter(
      (option) => option.value === settings?.enrollment_vpn_step_optional,
    )[0] || vpnOptionalityOptions[0];
  const { control, handleSubmit } = useForm<Inputs>({
    defaultValues: useMemo(() => {
      return {
        enrollment_vpn_step_optional: defaultVpnOptionality,
        enrollment_welcome_message: settings?.enrollment_welcome_message,
        enrollment_welcome_email: settings?.enrollment_welcome_email,
        enrollment_use_welcome_message_as_email:
          settings?.enrollment_use_welcome_message_as_email,
      };
    }, [
      defaultVpnOptionality,
      settings?.enrollment_welcome_message,
      settings?.enrollment_welcome_email,
      settings?.enrollment_use_welcome_message_as_email,
    ]),
    resolver: yupResolver(formSchema),
    mode: 'all',
  });

  useEffect(() => {
    if (settings) {
      setWelcomeMessage(settings.enrollment_welcome_message);
    }
  }, [settings, settings?.enrollment_welcome_message]);
  useEffect(() => {
    if (settings) {
      setWelcomeEmail(settings.enrollment_welcome_email);
    }
  }, [settings, settings?.enrollment_welcome_email]);

  if (!settings) return null;

  const onSubmit: SubmitHandler<Inputs> = (data) => {
    settings.enrollment_vpn_step_optional = data.enrollment_vpn_step_optional.value;
    settings.enrollment_welcome_message = welcomeMessage;
    settings.enrollment_welcome_email = welcomeEmail;
    settings.enrollment_use_welcome_message_as_email =
      data.enrollment_use_welcome_message_as_email;
    mutate(settings);
  };

  return (
    <section className="enrollment">
      <MessageBox type={MessageBoxType.WARNING} className="explanation-message">
        <p>{LL.settingsPage.enrollment.helper()}</p>
      </MessageBox>
      <div className="controls">
        <Button
          form="enrollment-form"
          text={
            breakpoint !== 'mobile'
              ? LL.settingsPage.enrollment.form.controls.submit()
              : undefined
          }
          icon={<IconCheckmarkWhite />}
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.SAVE}
          loading={isLoading}
          type="submit"
        />
      </div>
      <form id="enrollment-form" onSubmit={handleSubmit(onSubmit)}>
        <div className="left">
          <section className="vpn-optionality">
            <Card>
              <header>
                <h2>{LL.settingsPage.enrollment.vpnOptionality.header()}</h2>
                <Helper>
                  {parse(LL.settingsPage.enrollment.vpnOptionality.helper())}
                </Helper>
              </header>
              <FormSelect
                styleVariant={SelectStyleVariant.WHITE}
                options={vpnOptionalityOptions}
                controller={{ control, name: 'enrollment_vpn_step_optional' }}
              />
            </Card>
          </section>
          <section className="welcome-message">
            <Card>
              <header>
                <h2>{LL.settingsPage.enrollment.welcomeMessage.header()}</h2>
                <Helper>
                  {parse(LL.settingsPage.enrollment.welcomeMessage.helper())}
                </Helper>
              </header>
              <MessageBox>
                <p>{LL.settingsPage.enrollment.form.welcomeMessage.helper()}</p>
              </MessageBox>
              <textarea
                value={welcomeMessage}
                onChange={(e) => setWelcomeMessage(e.target.value)}
                disabled={isLoading}
              />
            </Card>
          </section>
        </div>
        <div className="right">
          <section className="welcome-email">
            <Card>
              <header>
                <h2>{LL.settingsPage.enrollment.welcomeEmail.header()}</h2>
                <Helper>{parse(LL.settingsPage.enrollment.welcomeEmail.helper())}</Helper>
              </header>
              <MessageBox>
                <p>{LL.settingsPage.enrollment.form.welcomeEmail.helper()}</p>
              </MessageBox>
              <FormCheckBox
                disabled={isLoading}
                label={LL.settingsPage.enrollment.form.useMessageAsEmail.label()}
                controller={{ control, name: 'enrollment_use_welcome_message_as_email' }}
              />
              <textarea
                value={welcomeEmail}
                onChange={(e) => setWelcomeEmail(e.target.value)}
                disabled={isLoading}
              />
            </Card>
          </section>
        </div>
      </form>
    </section>
  );
};
