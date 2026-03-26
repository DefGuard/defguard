import { useMutation, useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import type { Settings } from '../../../shared/api/types';
import { Controls } from '../../../shared/components/Controls/Controls';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { MarkedSection } from '../../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { MarkedSectionHeader } from '../../../shared/defguard-ui/components/MarkedSectionHeader/MarkedSectionHeader';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { getSettingsQueryOptions } from '../../../shared/query';
import {
  createNumericSelectOptions,
  withNumericFallbackOption,
} from '../../../shared/utils/numericSelectOptions';

const enrollmentTokenTimeoutBaseOptions = createNumericSelectOptions({
  1: m.settings_duration_one_hour(),
  12: m.settings_duration_hours({ hours: 12 }),
  24: m.settings_duration_one_day(),
  168: m.settings_duration_one_week(),
});

const enrollmentSessionTimeoutBaseOptions = createNumericSelectOptions({
  10: m.settings_duration_minutes({ minutes: 10 }),
  30: m.settings_duration_minutes({ minutes: 30 }),
  60: m.settings_duration_one_hour(),
});

const generalTabFormSchema = z.object({
  enrollment_token_timeout_hours: z.number(m.form_error_required()).int().min(1),
  enrollment_session_timeout_minutes: z.number(m.form_error_required()).int().min(1),
});

type GeneralTabFormFields = z.infer<typeof generalTabFormSchema>;

export const GeneralTab = () => {
  const { data: settings } = useQuery(getSettingsQueryOptions);

  return (
    <SettingsLayout>
      <SettingsHeader
        icon="key"
        title={m.settings_enrollment_general_title()}
        subtitle={m.settings_enrollment_page_subtitle()}
      />
      <SizedBox height={ThemeSpacing.Lg} />
      {isPresent(settings) && <GeneralTabContent settings={settings} />}
    </SettingsLayout>
  );
};

const GeneralTabContent = ({ settings }: { settings: Settings }) => {
  const { mutateAsync } = useMutation({
    mutationFn: api.settings.patchSettings,
    meta: {
      invalidate: ['settings'],
    },
    onSuccess: () => {
      Snackbar.default(m.settings_msg_saved());
    },
    onError: () => {
      Snackbar.error(m.settings_msg_save_failed());
    },
  });

  const defaultValues = useMemo(
    (): GeneralTabFormFields => ({
      enrollment_token_timeout_hours: settings.enrollment_token_timeout_hours ?? 24,
      enrollment_session_timeout_minutes:
        settings.enrollment_session_timeout_minutes ?? 10,
    }),
    [settings],
  );

  const enrollmentTokenTimeoutOptions = useMemo(
    () =>
      withNumericFallbackOption(
        enrollmentTokenTimeoutBaseOptions,
        defaultValues.enrollment_token_timeout_hours,
        'hours',
      ),
    [defaultValues.enrollment_token_timeout_hours],
  );

  const enrollmentSessionTimeoutOptions = useMemo(
    () =>
      withNumericFallbackOption(
        enrollmentSessionTimeoutBaseOptions,
        defaultValues.enrollment_session_timeout_minutes,
        'minutes',
      ),
    [defaultValues.enrollment_session_timeout_minutes],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: generalTabFormSchema,
      onChange: generalTabFormSchema,
    },
    onSubmit: async ({ value }) => {
      await mutateAsync(value);
      form.reset(value);
    },
  });

  return (
    <SettingsCard>
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <MarkedSection icon="settings">
            <MarkedSectionHeader
              title={m.settings_enrollment_section_duration_title()}
              description={m.settings_enrollment_section_duration_description()}
            />
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
            <form.AppField name="enrollment_session_timeout_minutes">
              {(field) => (
                <field.FormSelect
                  required
                  label={m.settings_enrollment_label_session_expires_in()}
                  options={enrollmentSessionTimeoutOptions}
                />
              )}
            </form.AppField>
          </MarkedSection>
        </form.AppForm>
        <form.Subscribe
          selector={(state) => ({
            isDefault: state.isDefaultValue || state.isPristine,
            isSubmitting: state.isSubmitting,
            canSubmit: state.canSubmit,
          })}
        >
          {({ isDefault, isSubmitting, canSubmit }) => (
            <Controls>
              <div className="right">
                <Button
                  variant="primary"
                  text={m.controls_save_changes()}
                  disabled={isDefault || !canSubmit}
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
    </SettingsCard>
  );
};
