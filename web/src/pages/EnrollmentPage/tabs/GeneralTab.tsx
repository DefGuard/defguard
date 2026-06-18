import { useMutation, useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import type { LicenseInfo, Settings } from '../../../shared/api/types';
import { businessBadgeProps } from '../../../shared/components/badges/BusinessBadge';
import {
  ContextualHelpKey,
  ContextualHelpSidebar,
} from '../../../shared/components/ContextualHelp';
import { Controls } from '../../../shared/components/Controls/Controls';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { MarkedSection } from '../../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { MarkedSectionHeader } from '../../../shared/defguard-ui/components/MarkedSectionHeader/MarkedSectionHeader';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { openModal } from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';
import {
  getEnterpriseSettingsQueryOptions,
  getLicenseInfoQueryOptions,
  getSettingsQueryOptions,
} from '../../../shared/query';
import { canUseBusinessFeature } from '../../../shared/utils/license';
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
  display_download_step: z.boolean(),
  display_password_reset: z.boolean(),
});

type GeneralTabFormFields = z.infer<typeof generalTabFormSchema>;

export const GeneralTab = () => {
  const { data: settings } = useQuery(getSettingsQueryOptions);
  const { data: license, isFetched } = useQuery(getLicenseInfoQueryOptions);

  return (
    <SettingsLayout
      suggestion={<ContextualHelpSidebar pageKey={ContextualHelpKey.EnrollmentGeneral} />}
    >
      <SettingsHeader
        icon="key"
        title={m.settings_enrollment_general_title()}
        subtitle={m.settings_enrollment_page_subtitle()}
        badgeProps={!isPresent(license) && isFetched ? businessBadgeProps : undefined}
      />
      <SizedBox height={ThemeSpacing.Lg} />
      {isPresent(settings) && <GeneralTabContent settings={settings} license={license} />}
    </SettingsLayout>
  );
};

const GeneralTabContent = ({
  settings,
  license,
}: {
  settings: Settings;
  license: LicenseInfo | null;
}) => {
  const noLicense = !isPresent(license);
  const { data: enterpriseSettings } = useQuery(getEnterpriseSettingsQueryOptions);

  const { mutateAsync: patchSettings } = useMutation({
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

  const { mutateAsync: patchEnterpriseSettings } = useMutation({
    mutationFn: api.settings.patchEnterpriseSettings,
    meta: {
      invalidate: [['settings_enterprise'], ['settings']],
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
      display_download_step: enterpriseSettings?.display_download_step ?? true,
      display_password_reset: enterpriseSettings?.display_password_reset ?? true,
    }),
    [settings, enterpriseSettings],
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
      // Always patch open-source settings
      await patchSettings({
        enrollment_token_timeout_hours: value.enrollment_token_timeout_hours,
        enrollment_session_timeout_minutes: value.enrollment_session_timeout_minutes,
      } as Partial<Settings>);
      // Patch enterprise settings only when license is active
      if (license) {
        const { result } = canUseBusinessFeature(license);
        if (result) {
          await patchEnterpriseSettings({
            display_download_step: value.display_download_step,
            display_password_reset: value.display_password_reset,
          });
        } else {
          openModal(ModalName.LicenseExpired, {
            licenseTier: license.tier,
          });
        }
      }
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
            <h3>{m.settings_enrollment_section_general_title()}</h3>
            <DescriptionBlock title={m.settings_enrollment_display_download_title()}>
              <p>{m.settings_enrollment_display_download_description()}</p>
            </DescriptionBlock>
            <form.AppField name="display_download_step">
              {(field) => (
                <field.FormInteractiveBlock
                  disabled={noLicense}
                  variant="toggle"
                  title={m.settings_enrollment_display_download_label()}
                  content={m.settings_enrollment_display_download_content()}
                />
              )}
            </form.AppField>
            <DescriptionBlock
              title={m.settings_enrollment_display_password_reset_title()}
            >
              <p>{m.settings_enrollment_display_password_reset_description()}</p>
            </DescriptionBlock>
            <form.AppField name="display_password_reset">
              {(field) => (
                <field.FormInteractiveBlock
                  disabled={noLicense}
                  variant="toggle"
                  title={m.settings_enrollment_display_password_reset_label()}
                  content={m.settings_enrollment_display_password_reset_content()}
                />
              )}
            </form.AppField>
          </MarkedSection>
          <Divider spacing={ThemeSpacing.Xl2} />
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
                  helper={m.settings_enrollment_helper_token_timeout()}
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
                  helper={m.settings_enrollment_helper_session_timeout()}
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
