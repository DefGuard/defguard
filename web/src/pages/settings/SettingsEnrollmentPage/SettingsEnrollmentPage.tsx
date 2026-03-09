import { useMutation, useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { useMemo } from 'react';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import type { Settings } from '../../../shared/api/types';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import { Controls } from '../../../shared/components/Controls/Controls';
import { Page } from '../../../shared/components/Page/Page';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { getSettingsQueryOptions } from '../../../shared/query';
import {
  createNumericSelectOptions,
  withNumericFallbackOption,
} from '../../../shared/utils/numericSelectOptions';

const breadcrumbs = [
  <Link
    to="/settings"
    search={{
      tab: 'general',
    }}
    key={0}
  >
    {m.settings_breadcrumb_general()}
  </Link>,
  <Link to="/settings/enrollment" key={1}>
    {m.settings_breadcrumb_enrollment()}
  </Link>,
];

export const SettingsEnrollmentPage = () => {
  const { data: settings } = useQuery(getSettingsQueryOptions);
  return (
    <Page title={m.settings_page_title()}>
      <Breadcrumbs links={breadcrumbs} />
      <SettingsLayout>
        <SettingsHeader
          icon="customize"
          title={m.settings_enrollment_title()}
          subtitle={m.settings_enrollment_subtitle()}
        />
        {isPresent(settings) && (
          <SettingsCard>
            <Content settings={settings} />
          </SettingsCard>
        )}
      </SettingsLayout>
    </Page>
  );
};

const formSchema = z.object({
  enrollment_token_timeout_hours: z.number(m.form_error_required()).int().min(1),
  password_reset_token_timeout_hours: z.number(m.form_error_required()).int().min(1),
  enrollment_session_timeout_minutes: z.number(m.form_error_required()).int().min(1),
  password_reset_session_timeout_minutes: z.number(m.form_error_required()).int().min(1),
});

type FormFields = z.infer<typeof formSchema>;

const formatEnrollmentTokenTimeoutLabel = (value: number) => {
  switch (value) {
    case 24:
      return m.settings_duration_one_day();
    case 168:
      return m.settings_duration_one_week();
    case 1:
      return m.settings_duration_one_hour();
    default:
      return m.settings_duration_hours({ hours: value });
  }
};

const enrollmentTokenTimeoutBaseOptions = createNumericSelectOptions(
  [1, 12, 24, 168],
  formatEnrollmentTokenTimeoutLabel,
);

const formatEnrollmentSessionTimeoutLabel = (value: number) => {
  switch (value) {
    case 60:
      return m.settings_duration_one_hour();
    case 1:
      return m.settings_duration_one_minute();
    default:
      return m.settings_duration_minutes({ minutes: value });
  }
};

const enrollmentSessionTimeoutBaseOptions = createNumericSelectOptions(
  [10, 30, 60],
  formatEnrollmentSessionTimeoutLabel,
);

const Content = ({ settings }: { settings: Settings }) => {
  const { mutateAsync } = useMutation({
    mutationFn: api.settings.patchSettings,
    meta: {
      invalidate: ['settings'],
    },
  });

  const defaultValues = useMemo(
    (): FormFields => ({
      enrollment_token_timeout_hours: settings.enrollment_token_timeout_hours ?? 24,
      password_reset_token_timeout_hours:
        settings.password_reset_token_timeout_hours ?? 24,
      enrollment_session_timeout_minutes:
        settings.enrollment_session_timeout_minutes ?? 10,
      password_reset_session_timeout_minutes:
        settings.password_reset_session_timeout_minutes ?? 10,
    }),
    [settings],
  );

  const enrollmentTokenTimeoutOptions = useMemo(
    () =>
      withNumericFallbackOption(
        enrollmentTokenTimeoutBaseOptions,
        defaultValues.enrollment_token_timeout_hours,
        formatEnrollmentTokenTimeoutLabel,
      ),
    [defaultValues.enrollment_token_timeout_hours],
  );

  const passwordResetTokenTimeoutOptions = useMemo(
    () =>
      withNumericFallbackOption(
        enrollmentTokenTimeoutBaseOptions,
        defaultValues.password_reset_token_timeout_hours,
        formatEnrollmentTokenTimeoutLabel,
      ),
    [defaultValues.password_reset_token_timeout_hours],
  );

  const enrollmentSessionTimeoutOptions = useMemo(
    () =>
      withNumericFallbackOption(
        enrollmentSessionTimeoutBaseOptions,
        defaultValues.enrollment_session_timeout_minutes,
        formatEnrollmentSessionTimeoutLabel,
      ),
    [defaultValues.enrollment_session_timeout_minutes],
  );

  const passwordResetSessionTimeoutOptions = useMemo(
    () =>
      withNumericFallbackOption(
        enrollmentSessionTimeoutBaseOptions,
        defaultValues.password_reset_session_timeout_minutes,
        formatEnrollmentSessionTimeoutLabel,
      ),
    [defaultValues.password_reset_session_timeout_minutes],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      await mutateAsync(value);
      form.reset(value);
    },
  });

  return (
    <form
      onSubmit={(e) => {
        e.stopPropagation();
        e.preventDefault();
        form.handleSubmit();
      }}
    >
      <form.AppForm>
        <form.AppField name="enrollment_token_timeout_hours">
          {(field) => (
            <field.FormSelect
              required
              label={m.settings_enrollment_label_token_validity()}
              options={enrollmentTokenTimeoutOptions}
            />
          )}
        </form.AppField>
        <SizedBox height={ThemeSpacing.Xl} />
        <form.AppField name="password_reset_token_timeout_hours">
          {(field) => (
            <field.FormSelect
              required
              label={m.settings_enrollment_label_password_reset_token_validity()}
              options={passwordResetTokenTimeoutOptions}
            />
          )}
        </form.AppField>
        <SizedBox height={ThemeSpacing.Xl} />
        <form.AppField name="enrollment_session_timeout_minutes">
          {(field) => (
            <field.FormSelect
              required
              label={m.settings_enrollment_label_session_expires_in()}
              options={enrollmentSessionTimeoutOptions}
            />
          )}
        </form.AppField>
        <SizedBox height={ThemeSpacing.Xl} />
        <form.AppField name="password_reset_session_timeout_minutes">
          {(field) => (
            <field.FormSelect
              required
              label={m.settings_enrollment_label_password_reset_session_expires_in()}
              options={passwordResetSessionTimeoutOptions}
            />
          )}
        </form.AppField>
      </form.AppForm>
      <form.Subscribe
        selector={(s) => ({
          isDefault: s.isDefaultValue || s.isPristine,
          isSubmitting: s.isSubmitting,
        })}
      >
        {({ isDefault, isSubmitting }) => (
          <Controls>
            <div className="right">
              <Button
                variant="primary"
                text={m.controls_save_changes()}
                disabled={isDefault}
                loading={isSubmitting}
                type="submit"
                onClick={() => {
                  form.handleSubmit();
                }}
              />
            </div>
          </Controls>
        )}
      </form.Subscribe>
    </form>
  );
};
