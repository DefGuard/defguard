import { m } from '../../../../paraglide/messages';
import type { SmtpEncryptionValue } from '../../../../shared/api/types';
import { SmtpEncryption } from '../../../../shared/api/types';
import type { SelectOption } from '../../../../shared/defguard-ui/components/Select/types';

const encryptionValueToLabel = (value: SmtpEncryptionValue): string => {
  switch (value) {
    case 'ImplicitTls':
      return m.settings_smtp_encryption_implicit_tls();
    case 'StartTls':
      return m.settings_smtp_encryption_start_tls();
    case 'None':
      return m.settings_smtp_encryption_none();
  }
};

export const encryptionSelectOptions: SelectOption<SmtpEncryptionValue>[] = Object.values(
  SmtpEncryption,
).map((e) => ({ key: e, label: encryptionValueToLabel(e), value: e }));
