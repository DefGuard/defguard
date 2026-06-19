import z from 'zod';
import { m } from '../../../../paraglide/messages';
import { SmtpAuthentication, SmtpEncryption } from '../../../../shared/api/types';
import { EvenSplit } from '../../../../shared/defguard-ui/components/EvenSplit/EvenSplit';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { useAppForm } from '../../../../shared/form';
import { formChangeLogic } from '../../../../shared/formLogic';
import { patternValidEmail } from '../../../../shared/patterns';
import { isMicrosoftIssuerUrl } from '../smtpAuthUtils';
import {
  MICROSOFT_ISSUER_URL,
  MICROSOFT_SMTP_SERVER,
  PROVIDER_SMTP_PORT,
} from './oauthFlow';
import type { FormProps } from './types';

const schema = z.object({
  smtp_sender: z
    .string()
    .trim()
    .min(1, m.form_error_required())
    .regex(patternValidEmail, m.form_error_email()),
  smtp_oauth_tenant_id: z.string().trim().nullable(),
  smtp_oauth_client_id: z.string().trim().nullable(),
  smtp_oauth_client_secret: z.string().trim().nullable(),
});

export const MicrosoftAuthForm = ({ initialValues, onApply, onClose }: FormProps) => {
  const form = useAppForm({
    defaultValues: {
      smtp_sender: initialValues.smtp_sender,
      smtp_oauth_tenant_id: initialValues.smtp_oauth_tenant_id,
      smtp_oauth_client_id: initialValues.smtp_oauth_client_id,
      smtp_oauth_client_secret: initialValues.smtp_oauth_client_secret,
    },
    validationLogic: formChangeLogic,
    validators: { onSubmit: schema, onChange: schema },
    onSubmit: async ({ value }) => {
      await onApply({
        authentication: SmtpAuthentication.XOAuth2,
        smtp_sender: value.smtp_sender,
        smtp_server: MICROSOFT_SMTP_SERVER,
        smtp_port: PROVIDER_SMTP_PORT,
        smtp_encryption: SmtpEncryption.StartTls,
        smtp_oauth_issuer_url: isMicrosoftIssuerUrl(initialValues.smtp_oauth_issuer_url)
          ? initialValues.smtp_oauth_issuer_url
          : MICROSOFT_ISSUER_URL,
        smtp_oauth_tenant_id: value.smtp_oauth_tenant_id,
        smtp_oauth_client_id: value.smtp_oauth_client_id,
        smtp_oauth_client_secret: value.smtp_oauth_client_secret,
      });
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
        <EvenSplit>
          <form.AppField name="smtp_sender">
            {(field) => (
              <field.FormInput
                required
                label={m.settings_smtp_label_sender_email_address()}
                helper={m.settings_smtp_helper_sender_email_address()}
              />
            )}
          </form.AppField>
          <form.AppField name="smtp_oauth_tenant_id">
            {(field) => (
              <field.FormInput
                required
                label={m.settings_smtp_label_oauth_tenant_id()}
                helper={m.settings_smtp_helper_oauth_tenant_id()}
              />
            )}
          </form.AppField>
        </EvenSplit>
        <SizedBox height={ThemeSpacing.Xl} />
        <EvenSplit>
          <form.AppField name="smtp_oauth_client_id">
            {(field) => (
              <field.FormInput
                required
                label={m.settings_smtp_label_oauth_client_id()}
                helper={m.settings_smtp_helper_oauth_client_id()}
              />
            )}
          </form.AppField>
          <form.AppField name="smtp_oauth_client_secret">
            {(field) => (
              <field.FormInput
                required
                label={m.settings_smtp_label_oauth_client_secret()}
                helper={m.settings_smtp_helper_oauth_client_secret()}
                type="password"
              />
            )}
          </form.AppField>
        </EvenSplit>
        <form.Subscribe selector={(s) => ({ isSubmitting: s.isSubmitting })}>
          {({ isSubmitting }) => (
            <ModalControls
              submitProps={{
                testId: "submit",
                text: m.controls_submit(),
                loading: isSubmitting,
                onClick: () => form.handleSubmit(),
              }}
              cancelProps={{
                testId: "cancel",
                text: m.controls_cancel(),
                disabled: isSubmitting,
                onClick: onClose,
              }}
            />
          )}
        </form.Subscribe>
      </form.AppForm>
    </form>
  );
};
