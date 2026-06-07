import { useMemo } from 'react';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import type { SmtpAuthenticationValue } from '../../../shared/api/types';
import { SmtpAuthentication } from '../../../shared/api/types';
import { EvenSplit } from '../../../shared/defguard-ui/components/EvenSplit/EvenSplit';
import { Modal } from '../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import type { SmtpAuthCardVariant } from './components/SmtpAuthMethodCard/SmtpAuthMethodCard';

export type SmtpAuthModalValues = {
  smtp_user: string | null;
  smtp_password: string | null;
  smtp_oauth_issuer_url: string | null;
  smtp_oauth_client_id: string | null;
  smtp_oauth_client_secret: string | null;
};

export type SmtpAuthApplyResult = {
  authentication: SmtpAuthenticationValue;
  values: SmtpAuthModalValues;
};

type Props = {
  isOpen: boolean;
  variant: SmtpAuthCardVariant | null;
  initialValues: SmtpAuthModalValues;
  onApply: (result: SmtpAuthApplyResult) => void;
  onClose: () => void;
};

const GOOGLE_ISSUER_URL = 'https://accounts.google.com';
const MICROSOFT_ISSUER_URL_DEFAULT = 'https://outlook.office.com/SMTP.Send';

const modalTitles: Record<SmtpAuthCardVariant, () => string> = {
  basic: m.settings_smtp_auth_modal_basic_title,
  custom: m.settings_smtp_auth_modal_custom_title,
  google: m.settings_smtp_auth_modal_google_title,
  microsoft: m.settings_smtp_auth_modal_microsoft_title,
};

export const SmtpAuthConfigModal = ({
  isOpen,
  variant,
  initialValues,
  onApply,
  onClose,
}: Props) => {
  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={variant ? modalTitles[variant]() : ''}
      size="primary"
    >
      {variant && (
        <ModalContent
          key={variant}
          variant={variant}
          initialValues={initialValues}
          onApply={onApply}
          onClose={onClose}
        />
      )}
    </Modal>
  );
};

const formSchema = z.object({
  smtp_user: z.string().trim().nullable(),
  smtp_password: z.string().trim().nullable(),
  smtp_oauth_issuer_url: z.string().trim().nullable(),
  smtp_oauth_client_id: z.string().trim().nullable(),
  smtp_oauth_client_secret: z.string().trim().nullable(),
});

type ModalContentProps = {
  variant: SmtpAuthCardVariant;
  initialValues: SmtpAuthModalValues;
  onApply: (result: SmtpAuthApplyResult) => void;
  onClose: () => void;
};

const ModalContent = ({
  variant,
  initialValues,
  onApply,
  onClose,
}: ModalContentProps) => {
  const defaultValues = useMemo(() => {
    switch (variant) {
      case 'basic':
        return {
          smtp_user: initialValues.smtp_user,
          smtp_password: initialValues.smtp_password,
          smtp_oauth_issuer_url: null,
          smtp_oauth_client_id: null,
          smtp_oauth_client_secret: null,
        };
      case 'custom':
        return {
          smtp_user: null,
          smtp_password: null,
          smtp_oauth_issuer_url: initialValues.smtp_oauth_issuer_url,
          smtp_oauth_client_id: initialValues.smtp_oauth_client_id,
          smtp_oauth_client_secret: initialValues.smtp_oauth_client_secret,
        };
      case 'google':
        return {
          smtp_user: null,
          smtp_password: null,
          smtp_oauth_issuer_url: GOOGLE_ISSUER_URL,
          smtp_oauth_client_id: initialValues.smtp_oauth_client_id,
          smtp_oauth_client_secret: initialValues.smtp_oauth_client_secret,
        };
      case 'microsoft':
        return {
          smtp_user: null,
          smtp_password: null,
          smtp_oauth_issuer_url: initialValues.smtp_oauth_issuer_url?.includes(
            'microsoftonline',
          )
            ? initialValues.smtp_oauth_issuer_url
            : MICROSOFT_ISSUER_URL_DEFAULT,
          smtp_oauth_client_id: initialValues.smtp_oauth_client_id,
          smtp_oauth_client_secret: initialValues.smtp_oauth_client_secret,
        };
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    variant,
    initialValues.smtp_password,
    initialValues.smtp_oauth_client_secret,
    initialValues.smtp_user,
    initialValues.smtp_oauth_issuer_url?.includes,
    initialValues.smtp_oauth_issuer_url,
    initialValues.smtp_oauth_client_id,
  ]);

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      const authentication =
        variant === 'basic' ? SmtpAuthentication.Login : SmtpAuthentication.XOAuth2;
      onApply({ authentication, values: value });
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
        {variant === 'basic' && (
          <EvenSplit>
            <form.AppField name="smtp_user">
              {(field) => (
                <field.FormInput
                  label={m.settings_smtp_label_server_username()}
                  helper={m.settings_smtp_helper_server_username()}
                />
              )}
            </form.AppField>
            <form.AppField name="smtp_password">
              {(field) => (
                <field.FormInput
                  label={m.settings_smtp_label_server_password()}
                  helper={m.settings_smtp_helper_server_password()}
                  type="password"
                />
              )}
            </form.AppField>
          </EvenSplit>
        )}

        {variant === 'custom' && (
          <>
            <form.AppField name="smtp_oauth_issuer_url">
              {(field) => (
                <field.FormInput
                  label={m.settings_smtp_label_oauth_issuer_url()}
                  helper={m.settings_smtp_helper_oauth_issuer_url()}
                />
              )}
            </form.AppField>
            <SizedBox height={ThemeSpacing.Xl} />
            <EvenSplit>
              <form.AppField name="smtp_oauth_client_id">
                {(field) => (
                  <field.FormInput
                    label={m.settings_smtp_label_oauth_client_id()}
                    helper={m.settings_smtp_helper_oauth_client_id()}
                  />
                )}
              </form.AppField>
              <form.AppField name="smtp_oauth_client_secret">
                {(field) => (
                  <field.FormInput
                    label={m.settings_smtp_label_oauth_client_secret()}
                    helper={m.settings_smtp_helper_oauth_client_secret()}
                    type="password"
                  />
                )}
              </form.AppField>
            </EvenSplit>
          </>
        )}

        {(variant === 'google' || variant === 'microsoft') && (
          <EvenSplit>
            <form.AppField name="smtp_oauth_client_id">
              {(field) => (
                <field.FormInput
                  label={m.settings_smtp_label_oauth_client_id()}
                  helper={m.settings_smtp_helper_oauth_client_id()}
                />
              )}
            </form.AppField>
            <form.AppField name="smtp_oauth_client_secret">
              {(field) => (
                <field.FormInput
                  label={m.settings_smtp_label_oauth_client_secret()}
                  helper={m.settings_smtp_helper_oauth_client_secret()}
                  type="password"
                />
              )}
            </form.AppField>
          </EvenSplit>
        )}

        <SizedBox height={ThemeSpacing.Xl2} />
        <form.Subscribe selector={(s) => ({ isSubmitting: s.isSubmitting })}>
          {({ isSubmitting }) => (
            <ModalControls
              submitProps={{
                text: m.controls_apply(),
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
