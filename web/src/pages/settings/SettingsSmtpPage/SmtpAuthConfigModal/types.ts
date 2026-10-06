import type {
  SecretValue,
  SmtpAuthenticationValue,
  SmtpEncryptionValue,
} from '../../../../shared/api/types';

export type SmtpAuthModalValues = {
  smtp_server: string;
  smtp_port: number;
  smtp_sender: string;
  smtp_encryption: SmtpEncryptionValue;
  smtp_tls_verify_cert: boolean;
  smtp_user: string | null;
  smtp_password?: SecretValue;
  smtp_password_set: boolean;
  smtp_oauth_issuer_url: string | null;
  smtp_oauth_client_id: string | null;
  smtp_oauth_client_secret?: SecretValue;
  smtp_oauth_client_secret_set: boolean;
  smtp_oauth_refresh_token?: SecretValue;
  smtp_oauth_tenant_id: string | null;
};

export type SmtpAuthApplyResult = {
  authentication: SmtpAuthenticationValue;
  smtp_server?: string;
  smtp_port?: number;
  smtp_sender: string;
  smtp_encryption?: SmtpEncryptionValue;
  smtp_tls_verify_cert?: boolean;
  smtp_user?: string | null;
  smtp_password?: SecretValue;
  smtp_oauth_issuer_url?: string | null;
  smtp_oauth_client_id?: string | null;
  smtp_oauth_client_secret?: SecretValue;
  smtp_oauth_refresh_token?: SecretValue;
  smtp_oauth_tenant_id?: string | null;
};

export type FormProps = {
  initialValues: SmtpAuthModalValues;
  onApply: (result: SmtpAuthApplyResult) => Promise<void>;
  onClose: () => void;
};
