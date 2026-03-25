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
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { Fold } from '../../../shared/defguard-ui/components/Fold/Fold';
import { Icon } from '../../../shared/defguard-ui/components/Icon';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { getSettingsQueryOptions } from '../../../shared/query';

const messageTemplatesHelpVariables = [
  ['{{ first_name }}', m.settings_enrollment_template_help_first_name()],
  ['{{ last_name }}', m.settings_enrollment_template_help_last_name()],
  ['{{ username }}', m.settings_enrollment_template_help_username()],
  ['{{ admin_first_name }}', m.settings_enrollment_template_help_admin_first_name()],
  ['{{ admin_last_name }}', m.settings_enrollment_template_help_admin_last_name()],
  ['{{ admin_phone }}', m.settings_enrollment_template_help_admin_phone()],
  ['{{ admin_email }}', m.settings_enrollment_template_help_admin_email()],
  ['{{ defguard_url }}', m.settings_enrollment_template_help_defguard_url()],
] as const;

const messageTemplatesHelpMarkdown = [
  ['#, ##, ###', m.settings_enrollment_template_help_markdown_headings(), 'medium'],
  ['*text*', m.settings_enrollment_template_help_markdown_italic()],
  ['**text**', m.settings_enrollment_template_help_markdown_bold()],
  ['***text***', m.settings_enrollment_template_help_markdown_bold_italic()],
  ['> text', m.settings_enrollment_template_help_markdown_blockquote()],
  ['- item or 1. item', m.settings_enrollment_template_help_markdown_lists()],
  ['`code`', m.settings_enrollment_template_help_markdown_inline_code()],
  ['```code```', m.settings_enrollment_template_help_markdown_code_block()],
  ['***', m.settings_enrollment_template_help_markdown_horizontal_line()],
  ['[text](url)', m.settings_enrollment_template_help_markdown_link()],
  ['| and ---', m.settings_enrollment_template_help_markdown_tables()],
  ['\\', m.settings_enrollment_template_help_markdown_escape()],
] as const;

const messageTemplatesFormSchema = z.object({
  enrollment_display_welcome_message: z.boolean(),
  enrollment_welcome_message: z.string(),
  enrollment_send_welcome_email: z.boolean(),
  enrollment_welcome_email_subject: z.string().min(1, m.form_error_required()),
  enrollment_use_welcome_message_as_email: z.boolean(),
  enrollment_welcome_email: z.string(),
});

type MessageTemplatesFormFields = z.infer<typeof messageTemplatesFormSchema>;

export const MessageTemplatesTab = () => {
  const { data: settings } = useQuery(getSettingsQueryOptions);

  if (!isPresent(settings)) {
    return null;
  }

  return <MessageTemplatesTabContent settings={settings} />;
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
      enrollment_display_welcome_message: true,
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
      const { enrollment_display_welcome_message: _displayWelcomeMessage, ...payload } =
        value;
      await mutateAsync(payload);
      form.reset(value);
    },
  });

  return (
    <SettingsLayout>
      <div data-testid="enrollment-tab-message-templates">
        <div>
          <SettingsHeader
            icon="activity-notes"
            title={m.settings_enrollment_message_templates_title()}
            subtitle={m.settings_enrollment_message_templates_subtitle()}
          />
          <SettingsCard>
            <form
              onSubmit={(e) => {
                e.stopPropagation();
                e.preventDefault();
                form.handleSubmit();
              }}
            >
              <form.AppForm>
                <div>
                  <form.AppField name="enrollment_display_welcome_message">
                    {(field) => (
                      <field.FormInteractiveBlock
                        variant="toggle"
                        title={m.settings_enrollment_template_display_message_title()}
                        content={m.settings_enrollment_template_display_message_description()}
                      >
                        <form.Subscribe
                          selector={(state) =>
                            state.values.enrollment_display_welcome_message
                          }
                        >
                          {() => (
                            <>
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
                            </>
                          )}
                        </form.Subscribe>
                      </field.FormInteractiveBlock>
                    )}
                  </form.AppField>
                </div>
                <Divider spacing={ThemeSpacing.Xl2} />
                <div>
                  <form.AppField name="enrollment_send_welcome_email">
                    {(field) => (
                      <field.FormInteractiveBlock
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
                              <div>
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
        </div>
        <MessageTemplatesHelpPanel />
      </div>
    </SettingsLayout>
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
