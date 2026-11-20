import './style.scss';
import dayjs from 'dayjs';
import { m } from '../../../../paraglide/messages';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Icon, IconKind } from '../../../../shared/defguard-ui/components/Icon';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { withFieldGroup } from '../../../../shared/form';
import { displayDate } from '../../../../shared/utils/displayDate';
import {
  ClientAuthMethod,
  type ClientCertificateError,
  type ClientCredentials,
  pemMarkers,
} from '../../consts';

export const clientAuthMethodFieldNames = {
  client_auth_method: 'client_auth_method',
} as const;

export const clientCertificateFieldNames = {
  client_certificate: 'client_certificate',
  client_private_key: 'client_private_key',
} as const;

type AuthMethod = ClientCredentials['client_auth_method'];

export const ClientAuthMethodField = withFieldGroup({
  defaultValues: { client_auth_method: ClientAuthMethod.Secret as AuthMethod },
  props: {} as { onChange?: (value: AuthMethod) => void },
  render: ({ group, onChange }) => (
    <group.AppField
      name="client_auth_method"
      listeners={{ onChange: ({ value }) => onChange?.(value) }}
    >
      {(field) => (
        <div className="client-auth-method">
          <field.FormRadio
            value={ClientAuthMethod.Secret}
            text={m.settings_openid_provider_auth_method_secret()}
          />
          <field.FormRadio
            value={ClientAuthMethod.Certificate}
            text={m.settings_openid_provider_auth_method_certificate()}
          />
        </div>
      )}
    </group.AppField>
  ),
});

const CertificateExpiry = ({ expiry }: { expiry: string }) =>
  dayjs.utc(expiry).isBefore(dayjs()) ? (
    <>
      <Icon
        icon={IconKind.WarningFilled}
        staticColor={ThemeVariable.FgCritical}
        size={16}
      />
      <AppText as="span" font={TextStyle.TBodyXs400} color={ThemeVariable.FgCritical}>
        {m.misc_expired()}
      </AppText>
    </>
  ) : (
    <span>
      {m.settings_certs_valid_until()} {displayDate(expiry, 'DD/MM/YYYY')}
    </span>
  );

type CertificateFieldsProps = {
  error: ClientCertificateError | null;
  onChange: () => void;
  expiry?: string | null;
};

export const ClientCertificateFields = withFieldGroup({
  defaultValues: { client_certificate: null, client_private_key: null } as Record<
    keyof typeof clientCertificateFieldNames,
    string | null | undefined
  >,
  props: {} as CertificateFieldsProps,
  render: ({ group, error, onChange, expiry }) => (
    <>
      <group.AppField name="client_certificate" listeners={{ onChange }}>
        {(field) => (
          <field.FormPemUploadField
            label={m.settings_openid_provider_certificate_label()}
            title={m.settings_openid_provider_upload_certificate()}
            acceptedExtensions={['.pem', '.crt', '.cer']}
            pemMarker={pemMarkers.client_certificate}
            error={error?.field === 'client_certificate' ? error.message : undefined}
            rowEnd={
              field.state.value === undefined && isPresent(expiry) ? (
                <CertificateExpiry expiry={expiry} />
              ) : undefined
            }
          />
        )}
      </group.AppField>
      <SizedBox height={ThemeSpacing.Lg} />
      <group.AppField name="client_private_key" listeners={{ onChange }}>
        {(field) => (
          <field.FormPemUploadField
            icon={IconKind.Key}
            label={m.settings_openid_provider_key_label()}
            title={m.settings_openid_provider_upload_key()}
            acceptedExtensions={['.pem', '.key']}
            pemMarker={pemMarkers.client_private_key}
            error={error?.field === 'client_private_key' ? error.message : undefined}
          />
        )}
      </group.AppField>
    </>
  ),
});
