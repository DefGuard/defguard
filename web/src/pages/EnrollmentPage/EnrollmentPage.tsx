import { useMutation, useQuery } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import z from 'zod';
import './style.scss';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import type { Settings } from '../../shared/api/types';
import { Controls } from '../../shared/components/Controls/Controls';
import { Page } from '../../shared/components/Page/Page';
import { SettingsCard } from '../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../shared/components/SettingsLayout/SettingsLayout';
import { AppText } from '../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { Fold } from '../../shared/defguard-ui/components/Fold/Fold';
import { Icon } from '../../shared/defguard-ui/components/Icon';
import { MarkedSection } from '../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { MarkedSectionHeader } from '../../shared/defguard-ui/components/MarkedSectionHeader/MarkedSectionHeader';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Tabs } from '../../shared/defguard-ui/components/Tabs/Tabs';
import type { TabsItem } from '../../shared/defguard-ui/components/Tabs/types';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../shared/defguard-ui/types';
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

const messageTemplatesHelpVariables = [
  ['{{ first_name }}', 'newly created user first name'],
  ['{{ last_name }}', 'newly created user last name'],
  ['{{ username }}', 'newly created user username/login'],
  [
    '{{ admin_first_name }}',
    'first name of the administrator who initiated the enrollment process',
  ],
  [
    '{{ admin_last_name }}',
    'last name of the administrator who initiated the enrollment process',
  ],
  [
    '{{ admin_phone }}',
    'phone number of the administrator who initiated the enrollment process',
  ],
  [
    '{{ admin_email }}',
    'email of the administrator who initiated the enrollment process',
  ],
  ['{{ defguard_url }}', 'internal Defguard URL (your Defguard instance address)'],
] as const;

const messageTemplatesHelpMarkdown = [
  ['#, ##, ###', 'Create headings.', 'medium'],
  ['*text*', 'Italic text.'],
  ['**text**', 'Bold text.'],
  ['***text***', 'Bold and italic.'],
  ['> text', 'Blockquote.'],
  ['- item or 1. item', 'Lists (unordered or ordered).'],
  ['`code`', 'Inline code.'],
  ['```code```', 'Code block.'],
  ['***', 'Horizontal line.'],
  ['[text](url)', 'Link.'],
  ['| and ---', 'Create tables.'],
  ['\\', 'Escape special characters.'],
] as const;

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
      <SettingsLayout>
        {activeTab === EnrollmentPageTab.General && (
          <>
            <SettingsHeader
              icon="key"
              title={m.settings_enrollment_general_title()}
              subtitle={m.settings_enrollment_page_subtitle()}
            />
            <SizedBox height={ThemeSpacing.Lg} />
            {isPresent(settings) && <GeneralTabContent settings={settings} />}
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
            {isPresent(settings) && <MessageTemplatesTabContent settings={settings} />}
          </>
        )}
      </SettingsLayout>
    </Page>
  );
};

const generalTabFormSchema = z.object({
  enrollment_token_timeout_hours: z.number(m.form_error_required()).int().min(1),
  enrollment_session_timeout_minutes: z.number(m.form_error_required()).int().min(1),
});

type GeneralTabFormFields = z.infer<typeof generalTabFormSchema>;

const messageTemplatesFormSchema = z.object({
  enrollment_welcome_message: z.string(),
  enrollment_send_welcome_email: z.boolean(),
  enrollment_welcome_email_subject: z.string().min(1, m.form_error_required()),
  enrollment_use_welcome_message_as_email: z.boolean(),
  enrollment_welcome_email: z.string(),
});

type MessageTemplatesFormFields = z.infer<typeof messageTemplatesFormSchema>;

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

const MessageTemplatesTabContent = ({ settings }: { settings: Settings }) => {
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
    (): MessageTemplatesFormFields => ({
      enrollment_welcome_message: settings.enrollment_welcome_message ?? '',
      enrollment_send_welcome_email: settings.enrollment_send_welcome_email ?? true,
      enrollment_welcome_email_subject: settings.enrollment_welcome_email_subject ?? '',
      enrollment_use_welcome_message_as_email:
        settings.enrollment_use_welcome_message_as_email ?? true,
      enrollment_welcome_email: settings.enrollment_welcome_email ?? '',
    }),
    [settings],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: messageTemplatesFormSchema,
      onChange: messageTemplatesFormSchema,
    },
    onSubmit: async ({ value }) => {
      await mutateAsync(value);
      form.reset(value);
    },
  });

  return (
    <div
      className="message-templates-layout"
      data-testid="enrollment-tab-message-templates"
    >
      <SettingsCard>
        <form
          onSubmit={(e) => {
            e.stopPropagation();
            e.preventDefault();
            form.handleSubmit();
          }}
        >
          <form.AppForm>
            <div className="message-template-section message-template-section-offset">
              <div className="message-template-offset-spacer" />
              <div className="message-template-offset-content">
                <div className="message-template-static-header">
                  <AppText
                    font={TextStyle.TBodyPrimary500}
                    color={ThemeVariable.FgDefault}
                  >
                    {m.settings_enrollment_template_display_message_title()}
                  </AppText>
                  <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgNeutral}>
                    {m.settings_enrollment_template_display_message_description()}
                  </AppText>
                </div>
                <SizedBox height={ThemeSpacing.Xl2} />
                <form.AppField name="enrollment_welcome_message">
                  {(field) => (
                    <field.FormTextarea
                      required
                      label={m.settings_enrollment_template_message_label()}
                      minHeight={383}
                      maxHeight={383}
                    />
                  )}
                </form.AppField>
              </div>
            </div>
            <div className="message-template-offset-divider">
              <Divider spacing={ThemeSpacing.Xl2} />
            </div>
            <div className="message-template-section">
              <form.AppField name="enrollment_send_welcome_email">
                {(field) => (
                  <field.FormInteractiveBlock
                    className="message-template-toggle"
                    variant="toggle"
                    title={m.settings_enrollment_template_send_email_title()}
                    content={m.settings_enrollment_template_send_email_description()}
                  >
                    <form.Subscribe
                      selector={(state) => state.values.enrollment_send_welcome_email}
                    >
                      {(sendWelcomeEmail) => (
                        <Fold open={sendWelcomeEmail}>
                          <SizedBox height={ThemeSpacing.Xl2} />
                          <form.AppField name="enrollment_welcome_email_subject">
                            {(field) => (
                              <field.FormInput
                                required
                                label={m.settings_enrollment_template_email_subject_label()}
                              />
                            )}
                          </form.AppField>
                          <SizedBox height={ThemeSpacing.Xl} />
                          <div className="message-templates-checkbox">
                            <form.AppField name="enrollment_use_welcome_message_as_email">
                              {(field) => (
                                <field.FormCheckbox
                                  text={m.settings_enrollment_template_same_as_message()}
                                />
                              )}
                            </form.AppField>
                          </div>
                          <SizedBox height={ThemeSpacing.Xl} />
                          <form.Subscribe
                            selector={(state) =>
                              state.values.enrollment_use_welcome_message_as_email
                            }
                          >
                            {(sameAsWelcomeMessage) => (
                              <>
                                <Fold open={sameAsWelcomeMessage}>
                                  <div className="message-templates-success-banner">
                                    <Icon
                                      icon="check-circle"
                                      staticColor={ThemeVariable.FgSuccess}
                                      size={20}
                                    />
                                    <p className="copy">
                                      {m.settings_enrollment_template_same_as_message_banner()}
                                    </p>
                                  </div>
                                </Fold>
                                <Fold open={!sameAsWelcomeMessage}>
                                  <SizedBox height={ThemeSpacing.Xl} />
                                  <form.AppField name="enrollment_welcome_email">
                                    {(field) => (
                                      <field.FormTextarea
                                        required
                                        label={m.settings_enrollment_template_email_label()}
                                        minHeight={383}
                                        maxHeight={383}
                                      />
                                    )}
                                  </form.AppField>
                                </Fold>
                              </>
                            )}
                          </form.Subscribe>
                        </Fold>
                      )}
                    </form.Subscribe>
                  </field.FormInteractiveBlock>
                )}
              </form.AppField>
            </div>
          </form.AppForm>
          <SizedBox height={ThemeSpacing.Xl2} />
          <Divider />
          <SizedBox height={ThemeSpacing.Xl} />
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
      <MessageTemplatesHelpPanel />
    </div>
  );
};

const MessageTemplatesHelpPanel = () => {
  return (
    <div className="message-templates-sidebar">
      <div className="sidebar-header">
        <Icon icon="info-outlined" staticColor={ThemeVariable.FgMuted} size={20} />
        <AppText font={TextStyle.TBodyPrimary600} color={ThemeVariable.FgDefault}>
          {m.settings_enrollment_template_help_title()}
        </AppText>
      </div>
      <div className="sidebar-panel">
        <ul className="sidebar-list">
          {messageTemplatesHelpVariables.map(([token, description]) => (
            <li key={token}>
              <span className="sidebar-token">{token}</span>
              <span className="sidebar-separator"> - </span>
              <span>{description}</span>
            </li>
          ))}
        </ul>
        <Divider />
        <ul className="sidebar-list">
          {messageTemplatesHelpMarkdown.map(([token, description, weight]) => (
            <li key={token}>
              <span
                className={weight === 'medium' ? 'sidebar-token-medium' : 'sidebar-token'}
              >
                {token}
              </span>
              <span className="sidebar-separator"> - </span>
              <span>{description}</span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
};
