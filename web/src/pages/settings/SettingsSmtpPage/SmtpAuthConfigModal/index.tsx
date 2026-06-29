import type { ComponentType } from 'react';
import { m } from '../../../../paraglide/messages';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import type { SmtpAuthCardVariant } from '../components/SmtpAuthMethodCard/SmtpAuthMethodCard';
import { BasicAuthForm } from './BasicAuthForm';
import { CustomAuthForm } from './CustomAuthForm';
import { GoogleAuthForm } from './GoogleAuthForm';
import { MicrosoftAuthForm } from './MicrosoftAuthForm';
import { NoneAuthForm } from './NoneAuthForm';
import type { FormProps, SmtpAuthApplyResult, SmtpAuthModalValues } from './types';

export type { SmtpAuthApplyResult, SmtpAuthModalValues } from './types';

type Props = {
  isOpen: boolean;
  variant: SmtpAuthCardVariant | null;
  initialValues: SmtpAuthModalValues;
  onApply: (result: SmtpAuthApplyResult) => Promise<void>;
  onClose: () => void;
};

const modalTitles: Record<SmtpAuthCardVariant, () => string> = {
  none: m.settings_smtp_auth_modal_none_title,
  basic: m.settings_smtp_auth_modal_basic_title,
  custom: m.settings_smtp_auth_modal_custom_title,
  google: m.settings_smtp_auth_modal_google_title,
  microsoft: m.settings_smtp_auth_modal_microsoft_title,
};

const forms: Record<SmtpAuthCardVariant, ComponentType<FormProps>> = {
  none: NoneAuthForm,
  basic: BasicAuthForm,
  google: GoogleAuthForm,
  microsoft: MicrosoftAuthForm,
  custom: CustomAuthForm,
};

export const SmtpAuthConfigModal = ({
  isOpen,
  variant,
  initialValues,
  onApply,
  onClose,
}: Props) => {
  const FormComponent = variant ? forms[variant] : null;
  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={variant ? modalTitles[variant]() : ''}
      size="primary"
    >
      {variant && FormComponent && (
        <FormComponent
          key={variant}
          initialValues={initialValues}
          onApply={onApply}
          onClose={onClose}
        />
      )}
    </Modal>
  );
};
