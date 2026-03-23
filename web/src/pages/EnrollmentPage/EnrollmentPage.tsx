import { useMutation, useQuery } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import {
  EnrollmentAdminEmailMode,
  EnrollmentReleaseChannel,
  type EnrollmentReleaseChannelValue,
  type Settings,
} from '../../shared/api/types';
import { Controls } from '../../shared/components/Controls/Controls';
import { Page } from '../../shared/components/Page/Page';
import { SettingsCard } from '../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../shared/components/SettingsLayout/SettingsLayout';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { Fold } from '../../shared/defguard-ui/components/Fold/Fold';
import { MarkedSection } from '../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { MarkedSectionHeader } from '../../shared/defguard-ui/components/MarkedSectionHeader/MarkedSectionHeader';
import type { SelectOption } from '../../shared/defguard-ui/components/Select/types';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Tabs } from '../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../shared/defguard-ui/components/Tabs/types';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../shared/form';
import { formChangeLogic } from '../../shared/formLogic';
import { getSettingsQueryOptions } from '../../shared/query';
import {
  createNumericSelectOptions,
  withNumericFallbackOption,
} from '../../shared/utils/numericSelectOptions';

const EnrollmentPageTab = {
  General: 'general',
  MessageTemplates: 'message-templates',
} as const;

type EnrollmentTabValue = (typeof EnrollmentPageTab)[keyof typeof EnrollmentPageTab];

const adminEmailModeOptions = [
  {
    value: EnrollmentAdminEmailMode.InitiatingAdmin,
    title: m.settings_enrollment_admin_email_mode_initiating_admin(),
  },
  {
    value: EnrollmentAdminEmailMode.Hidden,
    title: m.settings_enrollment_admin_email_mode_hidden(),
  },
  {
    value: EnrollmentAdminEmailMode.CustomEmail,
    title: m.settings_enrollment_admin_email_mode_custom(),
  },
] as const;

const releaseChannelOptions: SelectOption<EnrollmentReleaseChannelValue>[] = [
  {
    key: EnrollmentReleaseChannel.Stable,
    value: EnrollmentReleaseChannel.Stable,
    label: m.settings_enrollment_release_channel_stable(),
  },
  {
    key: EnrollmentReleaseChannel.Beta,
    value: EnrollmentReleaseChannel.Beta,
    label: m.settings_enrollment_release_channel_beta(),
  },
  {
    key: EnrollmentReleaseChannel.Alpha,
    value: EnrollmentReleaseChannel.Alpha,
    label: m.settings_enrollment_release_channel_alpha(),
  },
];

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

export const EnrollmentPage = () => {
  const [activeTab, setActiveTab] = useState<EnrollmentTabValue>(
    EnrollmentPageTab.General,
  );
  const { data: settings } = useQuery(getSettingsQueryOptions);

  const tabs = useMemo(
    (): TabsItem[] => [
      {
        title: m.settings_enrollment_tab_general(),
        active: activeTab === EnrollmentPageTab.General,
        onClick: () => {
          setActiveTab(EnrollmentPageTab.General);
        },
      },
      {
        title: m.settings_enrollment_tab_message_templates(),
        active: activeTab === EnrollmentPageTab.MessageTemplates,
        onClick: () => {
          setActiveTab(EnrollmentPageTab.MessageTemplates);
        },
      },
    ],
    [activeTab],
  );

  return (
    <Page id="enrollment-page" title={m.settings_enrollment_page_title()}>
      <SizedBox height={ThemeSpacing.Md} />
      <Tabs items={tabs} />
      <SizedBox height={ThemeSpacing.Xl2} />
      <SettingsLayout>
        {activeTab === EnrollmentPageTab.General && (
          <>
            <SettingsHeader
              icon="key"
              title={m.settings_enrollment_general_title()}
              subtitle={m.settings_enrollment_page_subtitle()}
            />
            <SizedBox height={ThemeSpacing.Lg} />
            {isPresent(settings) && (
              <SettingsCard>
                <GeneralTabContent settings={settings} />
              </SettingsCard>
            )}
          </>
        )}
        {activeTab === EnrollmentPageTab.MessageTemplates && (
          <>
            <SettingsHeader
              icon="activity-notes"
              title={m.settings_enrollment_message_templates_title()}
              subtitle={m.settings_enrollment_message_templates_subtitle()}
            />
            <SizedBox height={ThemeSpacing.Lg} />
            <SettingsCard>
              <div data-testid="enrollment-tab-message-templates" />
            </SettingsCard>
          </>
        )}
      </SettingsLayout>
    </Page>
  );
};

const generalTabFormSchema = z
  .object({
    enrollment_admin_email_mode: z.enum(EnrollmentAdminEmailMode),
    enrollment_admin_custom_email: z.string(),
    enrollment_windows_release_channel: z.enum(EnrollmentReleaseChannel),
    enrollment_linux_release_channel: z.enum(EnrollmentReleaseChannel),
    enrollment_macos_release_channel: z.enum(EnrollmentReleaseChannel),
    enrollment_token_timeout_hours: z.number(m.form_error_required()).int().min(1),
    enrollment_session_timeout_minutes: z.number(m.form_error_required()).int().min(1),
    enrollment_show_reset_password: z.boolean(),
  })
  .superRefine((values, ctx) => {
    if (values.enrollment_admin_email_mode === EnrollmentAdminEmailMode.CustomEmail) {
      const result = z
        .email(m.form_error_email())
        .min(1, m.form_error_required())
        .safeParse(values.enrollment_admin_custom_email);
      if (!result.success) {
        ctx.addIssue({
          code: 'custom',
          path: ['enrollment_admin_custom_email'],
          message: result.error.message,
        });
      }
    }
  });

type GeneralTabFormFields = z.infer<typeof generalTabFormSchema>;

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
      enrollment_admin_email_mode:
        settings.enrollment_admin_email_mode ?? EnrollmentAdminEmailMode.InitiatingAdmin,
      enrollment_admin_custom_email: settings.enrollment_admin_custom_email ?? '',
      enrollment_windows_release_channel:
        settings.enrollment_windows_release_channel ?? EnrollmentReleaseChannel.Stable,
      enrollment_linux_release_channel:
        settings.enrollment_linux_release_channel ?? EnrollmentReleaseChannel.Stable,
      enrollment_macos_release_channel:
        settings.enrollment_macos_release_channel ?? EnrollmentReleaseChannel.Stable,
      enrollment_token_timeout_hours: settings.enrollment_token_timeout_hours ?? 24,
      enrollment_session_timeout_minutes:
        settings.enrollment_session_timeout_minutes ?? 10,
      enrollment_show_reset_password: settings.enrollment_show_reset_password ?? true,
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
      await mutateAsync({
        ...value,
        enrollment_admin_custom_email:
          value.enrollment_admin_email_mode === EnrollmentAdminEmailMode.CustomEmail
            ? value.enrollment_admin_custom_email
            : null,
      });
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
        <MarkedSection icon="settings">
          <MarkedSectionHeader
            title={m.settings_enrollment_section_general_title()}
            description={m.settings_enrollment_section_general_description()}
          />
          {adminEmailModeOptions.map((option, index) => (
            <div key={option.value}>
              {index > 0 && <SizedBox height={ThemeSpacing.Lg} />}
              <form.AppField name="enrollment_admin_email_mode">
                {(field) => (
                  <field.FormInteractiveBlock
                    variant="radio"
                    value={option.value}
                    title={option.title}
                  >
                    {option.value === EnrollmentAdminEmailMode.CustomEmail && (
                      <form.Subscribe
                        selector={(state) =>
                          state.values.enrollment_admin_email_mode ===
                          EnrollmentAdminEmailMode.CustomEmail
                        }
                      >
                        {(isCustomEmailMode) => (
                          <Fold open={isCustomEmailMode}>
                            <SizedBox height={ThemeSpacing.Lg} />
                            <form.AppField name="enrollment_admin_custom_email">
                              {(field) => (
                                <field.FormInput
                                  required
                                  label={m.settings_enrollment_admin_email_custom_label()}
                                />
                              )}
                            </form.AppField>
                          </Fold>
                        )}
                      </form.Subscribe>
                    )}
                  </field.FormInteractiveBlock>
                )}
              </form.AppField>
            </div>
          ))}
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="activity-notes">
          <MarkedSectionHeader
            title={m.settings_enrollment_section_versions_title()}
            description={m.settings_enrollment_section_versions_description()}
          />
          <form.AppField name="enrollment_windows_release_channel">
            {(field) => (
              <field.FormSelect
                required
                label={m.settings_enrollment_windows_channel_label()}
                options={releaseChannelOptions}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="enrollment_linux_release_channel">
            {(field) => (
              <field.FormSelect
                required
                label={m.settings_enrollment_linux_channel_label()}
                options={releaseChannelOptions}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="enrollment_macos_release_channel">
            {(field) => (
              <field.FormSelect
                required
                label={m.settings_enrollment_macos_channel_label()}
                options={releaseChannelOptions}
              />
            )}
          </form.AppField>
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="lock-closed">
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
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="enrollment_show_reset_password">
            {(field) => (
              <field.FormInteractiveBlock
                variant="toggle"
                title={m.settings_enrollment_toggle_reset_password_title()}
                content={m.settings_enrollment_toggle_reset_password_description()}
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
  );
};
