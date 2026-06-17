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
import { encryptionSelectOptions } from './encryptionOptions';
import type { FormProps } from './types';

const schema = z.object({
  smtp_server: z.string().trim().min(1, m.form_error_required()),
  smtp_port: z.number(m.form_error_required()).max(65535, m.form_error_port_max()),
  smtp_sender: z
    .string()
    .trim()
    .min(1, m.form_error_required())
    .regex(patternValidEmail, m.form_error_email()),
  smtp_encryption: z.enum(SmtpEncryption),
});

export const NoneAuthForm = ({ initialValues, onApply, onClose }: FormProps) => {
  const form = useAppForm({
    defaultValues: {
      smtp_server: initialValues.smtp_server,
      smtp_port: initialValues.smtp_port,
      smtp_sender: initialValues.smtp_sender,
      smtp_encryption: initialValues.smtp_encryption,
    },
    validationLogic: formChangeLogic,
    validators: { onSubmit: schema, onChange: schema },
    onSubmit: async ({ value }) => {
      await onApply({
        authentication: SmtpAuthentication.None,
        smtp_server: value.smtp_server,
        smtp_port: value.smtp_port,
        smtp_sender: value.smtp_sender,
        smtp_encryption: value.smtp_encryption,
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
          <form.AppField name="smtp_server">
            {(field) => (
              <field.FormInput
                required
                label={m.settings_smtp_label_server_address()}
                helper={m.settings_smtp_helper_server_address()}
              />
            )}
          </form.AppField>
          <form.AppField name="smtp_port">
            {(field) => (
              <field.FormInput
                required
                label={m.settings_smtp_label_server_port()}
                helper={m.settings_smtp_helper_server_port()}
                type="number"
              />
            )}
          </form.AppField>
        </EvenSplit>
        <SizedBox height={ThemeSpacing.Xl} />
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
          <form.AppField name="smtp_encryption">
            {(field) => (
              <field.FormSelect
                required
                options={encryptionSelectOptions}
                label={m.settings_smtp_label_encryption()}
                helper={m.settings_smtp_helper_encryption()}
              />
            )}
          </form.AppField>
        </EvenSplit>
        <SizedBox height={ThemeSpacing.Xl2} />
        <form.Subscribe selector={(s) => ({ isSubmitting: s.isSubmitting })}>
          {({ isSubmitting }) => (
            <ModalControls
              submitProps={{
                text: m.controls_submit(),
                loading: isSubmitting,
                onClick: () => form.handleSubmit(),
              }}
              cancelProps={{
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
