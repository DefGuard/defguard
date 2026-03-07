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
import {
  createNumericSelectOptions,
  formatHourSelectLabel,
  formatMinuteSelectLabel,
  withNumericFallbackOption,
} from '../../../shared/const/numericSelectOptions';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { getSettingsQueryOptions } from '../../../shared/query';

const breadcrumbs = [
  <Link
    to="/settings"
    search={{
      tab: 'general',
    }}
    key={0}
  >
    General
  </Link>,
  <Link to="/settings/enrollment" key={1}>
    Enrollment
  </Link>,
];

export const SettingsEnrollmentPage = () => {
  const { data: settings } = useQuery(getSettingsQueryOptions);
  return (
    <Page title="Settings">
      <Breadcrumbs links={breadcrumbs} />
      <SettingsLayout>
        <SettingsHeader
          icon="customize"
          title="Enrollment"
          subtitle="Configure token and session timeouts for enrollment and password reset flows."
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

const enrollmentTokenTimeoutBaseOptions = createNumericSelectOptions(
  [1, 2, 4, 8, 12, 24, 48, 72, 168],
  formatHourSelectLabel,
);

const enrollmentSessionTimeoutBaseOptions = createNumericSelectOptions(
  [5, 10, 15, 30, 45, 60],
  formatMinuteSelectLabel,
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
        formatHourSelectLabel,
      ),
    [defaultValues.enrollment_token_timeout_hours],
  );

  const passwordResetTokenTimeoutOptions = useMemo(
    () =>
      withNumericFallbackOption(
        enrollmentTokenTimeoutBaseOptions,
        defaultValues.password_reset_token_timeout_hours,
        formatHourSelectLabel,
      ),
    [defaultValues.password_reset_token_timeout_hours],
  );

  const enrollmentSessionTimeoutOptions = useMemo(
    () =>
      withNumericFallbackOption(
        enrollmentSessionTimeoutBaseOptions,
        defaultValues.enrollment_session_timeout_minutes,
        formatMinuteSelectLabel,
      ),
    [defaultValues.enrollment_session_timeout_minutes],
  );

  const passwordResetSessionTimeoutOptions = useMemo(
    () =>
      withNumericFallbackOption(
        enrollmentSessionTimeoutBaseOptions,
        defaultValues.password_reset_session_timeout_minutes,
        formatMinuteSelectLabel,
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
              label="Enrollment token timeout (hours)"
              options={enrollmentTokenTimeoutOptions}
            />
          )}
        </form.AppField>
        <SizedBox height={ThemeSpacing.Xl} />
        <form.AppField name="password_reset_token_timeout_hours">
          {(field) => (
            <field.FormSelect
              required
              label="Password reset token timeout (hours)"
              options={passwordResetTokenTimeoutOptions}
            />
          )}
        </form.AppField>
        <SizedBox height={ThemeSpacing.Xl} />
        <form.AppField name="enrollment_session_timeout_minutes">
          {(field) => (
            <field.FormSelect
              required
              label="Enrollment session timeout (minutes)"
              options={enrollmentSessionTimeoutOptions}
            />
          )}
        </form.AppField>
        <SizedBox height={ThemeSpacing.Xl} />
        <form.AppField name="password_reset_session_timeout_minutes">
          {(field) => (
            <field.FormSelect
              required
              label="Password reset session timeout (minutes)"
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
