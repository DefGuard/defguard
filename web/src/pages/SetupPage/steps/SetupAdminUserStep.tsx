import './style.scss';

import { useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import clsx from 'clsx';
import { useMemo } from 'react';
import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Icon } from '../../../shared/defguard-ui/components/Icon';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useAppForm, withForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { SetupPageStep } from '../types';
import { useSetupWizardStore } from '../useSetupWizardStore';

type FormFields = StoreValues;

type StoreValues = {
  first_name: string;
  last_name: string;
  username: string;
  email: string;
  password: string;
};

const passwordRules = [
  {
    id: 'required',
    label: m.initial_setup_admin_user_password_rule_required_label(),
    message: m.initial_setup_admin_user_password_rule_required_message(),
    test: (value: string) => value.length > 0,
    apply: (schema: z.ZodString) =>
      schema.min(1, m.initial_setup_admin_user_password_rule_required_message()),
  },
  {
    id: 'min',
    label: m.initial_setup_admin_user_password_rule_min_label(),
    message: m.initial_setup_admin_user_password_rule_min_message(),
    test: (value: string) => value.length >= 8,
    apply: (schema: z.ZodString) =>
      schema.min(8, m.initial_setup_admin_user_password_rule_min_message()),
  },
  {
    id: 'number',
    label: m.initial_setup_admin_user_password_rule_number_label(),
    message: m.initial_setup_admin_user_password_rule_number_message(),
    test: (value: string) => /[0-9]/.test(value),
    apply: (schema: z.ZodString) =>
      schema.regex(/[0-9]/, m.initial_setup_admin_user_password_rule_number_message()),
  },
  {
    id: 'special',
    label: m.initial_setup_admin_user_password_rule_special_label(),
    message: m.initial_setup_admin_user_password_rule_special_message(),
    test: (value: string) => /[!@#$%^&*(),.?":{}|<>]/.test(value),
    apply: (schema: z.ZodString) =>
      schema.regex(
        /[!@#$%^&*(),.?":{}|<>]/,
        m.initial_setup_admin_user_password_rule_special_message(),
      ),
  },
  {
    id: 'lower',
    label: m.initial_setup_admin_user_password_rule_lower_label(),
    message: m.initial_setup_admin_user_password_rule_lower_message(),
    test: (value: string) => /[a-z]/.test(value),
    apply: (schema: z.ZodString) =>
      schema.regex(/[a-z]/, m.initial_setup_admin_user_password_rule_lower_message()),
  },
  {
    id: 'upper',
    label: m.initial_setup_admin_user_password_rule_upper_label(),
    message: m.initial_setup_admin_user_password_rule_upper_message(),
    test: (value: string) => /[A-Z]/.test(value),
    apply: (schema: z.ZodString) =>
      schema.regex(/[A-Z]/, m.initial_setup_admin_user_password_rule_upper_message()),
  },
];

const passwordSchema = passwordRules.reduce(
  (schema, rule) => rule.apply(schema),
  z.string(),
);

export const SetupAdminUserStep = () => {
  const setActiveStep = useSetupWizardStore((s) => s.setActiveStep);
  const defaultValues = useSetupWizardStore(
    useShallow(
      (s): FormFields => ({
        first_name: s.admin_first_name,
        last_name: s.admin_last_name,
        username: s.admin_username,
        email: s.admin_email,
        password: s.admin_password,
      }),
    ),
  );

  const formSchema = useMemo(
    () =>
      z.object({
        first_name: z
          .string()
          .min(1, m.initial_setup_admin_user_error_first_name_required()),
        last_name: z
          .string()
          .min(1, m.initial_setup_admin_user_error_last_name_required()),
        username: z.string().min(3, m.initial_setup_admin_user_error_username_min()),
        email: z
          .email(m.initial_setup_admin_user_error_email_invalid())
          .min(1, m.initial_setup_admin_user_error_email_required()),
        password: passwordSchema,
      }),
    [],
  );

  const { mutate, isPending } = useMutation({
    mutationFn: api.initial_setup.createAdminUser,
    meta: {
      invalidate: ['setupStatus'],
    },
    onSuccess: () => {
      setActiveStep(SetupPageStep.GeneralConfig);
    },
    onError: (error) => {
      Snackbar.error(m.initial_setup_admin_user_error_create_failed());
      console.error('Failed to create admin user:', error);
    },
  });

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: ({ value }) => {
      useSetupWizardStore.setState({
        admin_first_name: value.first_name,
        admin_last_name: value.last_name,
        admin_username: value.username,
        admin_email: value.email,
        admin_password: value.password,
      });
      mutate({
        first_name: value.first_name,
        last_name: value.last_name,
        username: value.username,
        email: value.email,
        password: value.password,
      });
    },
  });

  const handleNext = () => {
    form.handleSubmit();
  };

  return (
    <WizardCard>
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
        className="setup-admin-user"
      >
        <form.AppForm>
          <div className="admin-user-form-grid">
            <form.AppField name="first_name">
              {(field) => (
                <field.FormInput
                  required
                  label={m.initial_setup_admin_user_label_first_name()}
                  type="text"
                />
              )}
            </form.AppField>
            <form.AppField name="last_name">
              {(field) => (
                <field.FormInput
                  required
                  label={m.initial_setup_admin_user_label_last_name()}
                  type="text"
                />
              )}
            </form.AppField>
            <form.AppField name="username">
              {(field) => (
                <field.FormInput
                  required
                  label={m.initial_setup_admin_user_label_username()}
                  type="text"
                />
              )}
            </form.AppField>
            <form.AppField name="email">
              {(field) => (
                <field.FormInput
                  required
                  label={m.initial_setup_admin_user_label_email()}
                />
              )}
            </form.AppField>
            <div className="full-row">
              <form.AppField name="password">
                {(field) => (
                  <field.FormInput
                    required
                    label={m.initial_setup_admin_user_label_password()}
                    type="password"
                  />
                )}
              </form.AppField>
              <SizedBox height={ThemeSpacing.Xl} />
              <PasswordChecklist form={form} />
            </div>
          </div>
          <SizedBox height={ThemeSpacing.Xl} />
        </form.AppForm>
      </form>
      <ModalControls
        submitProps={{
          text: m.controls_continue(),
          onClick: handleNext,
          loading: isPending,
        }}
      />
    </WizardCard>
  );
};

const PasswordChecklist = withForm({
  defaultValues: {
    first_name: '',
    last_name: '',
    username: '',
    email: '',
    password: '',
  },
  render: ({ form }) => {
    const password = useStore(form.store, (state) => state.values.password ?? '');
    const isPristine = useStore(
      form.store,
      (state) => state.fieldMeta.password?.isPristine ?? true,
    );

    const checks = passwordRules.map((rule) => ({
      id: rule.id,
      label: rule.label,
      passed: rule.test(password),
    }));

    return (
      <div className="password-checklist">
        <p>{m.initial_setup_admin_user_password_checklist_title()}</p>
        <ul>
          {checks.map((item) => {
            const checked = !isPristine && item.passed;
            const iconKind = checked ? 'check-filled' : 'empty-point';

            return (
              <li
                key={item.id}
                className={clsx({
                  active: checked,
                })}
              >
                <Icon icon={iconKind} size={16} />
                <span>{item.label}</span>
              </li>
            );
          })}
        </ul>
      </div>
    );
  },
});
