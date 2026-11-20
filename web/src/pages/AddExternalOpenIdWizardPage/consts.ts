import z from 'zod';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import {
  getApiErrorMessage,
  getAuthErrorFromException,
} from '../../shared/api/apiErrorMessages';
import {
  type AddOpenIdProvider,
  DirectorySyncBehavior,
  type DirectorySyncBehaviorValue,
  DirectorySyncTarget,
  type DirectorySyncTargetValue,
  OpenIdProviderUsernameHandling,
  type OpenIdProviderUsernameHandlingValue,
  type TestDirectorySyncResponse,
  WebErrorCode,
} from '../../shared/api/types';
import type { JumpCloudRegion } from '../../shared/constants';
import type { SelectOption } from '../../shared/defguard-ui/components/Select/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { AddExternalProviderStep } from './types';
import { useAddExternalOpenIdStore } from './useAddExternalOpenIdStore';

export const ClientAuthMethod = {
  Secret: 'secret',
  Certificate: 'certificate',
} as const;

// `undefined` in a PEM field means the server already stores the file.
export const clientCredentialsSchema = z.object({
  client_auth_method: z.enum(ClientAuthMethod),
  client_certificate: z.string().nullable().optional(),
  client_private_key: z.string().nullable().optional(),
});

export type ClientCredentials = z.infer<typeof clientCredentialsSchema>;

export const pemMarkers = {
  client_certificate: '-----BEGIN CERTIFICATE-----',
  client_private_key: 'PRIVATE KEY-----',
} as const;

export const refineClientCertificate = (
  values: ClientCredentials,
  ctx: z.RefinementCtx,
) => {
  if (values.client_auth_method !== ClientAuthMethod.Certificate) return;
  if (values.client_certificate === null || values.client_private_key === null) {
    ctx.addIssue({
      code: 'custom',
      continue: true,
      message: m.settings_openid_provider_certificate_missing(),
      path: [
        values.client_private_key === null ? 'client_private_key' : 'client_certificate',
      ],
    });
  }
};

export type ClientCertificateError = {
  field: keyof typeof pemMarkers;
  message: string;
};

type CertificateErrorCode = Extract<WebErrorCode, `cert_${string}`>;

// A new certificate code fails the build until it gets a field here.
const certificateErrorField: Record<
  CertificateErrorCode,
  ClientCertificateError['field']
> = {
  [WebErrorCode.CertMissingCertPem]: 'client_certificate',
  [WebErrorCode.CertMissingKeyPem]: 'client_private_key',
  [WebErrorCode.CertInvalidCertOrKey]: 'client_private_key',
  [WebErrorCode.CertInvalidValidityPeriod]: 'client_certificate',
  [WebErrorCode.CertExpired]: 'client_certificate',
  [WebErrorCode.CertNotYetValid]: 'client_certificate',
  [WebErrorCode.CertParseError]: 'client_certificate',
};

const isCertificateErrorCode = (code?: string): code is CertificateErrorCode =>
  isPresent(code) && Object.hasOwn(certificateErrorField, code);

export const clientCertificateError = (e: unknown): ClientCertificateError | null => {
  const code = getAuthErrorFromException(e).authError;
  if (!isCertificateErrorCode(code)) return null;
  return { field: certificateErrorField[code], message: getApiErrorMessage(code) };
};

export const clientCredentialsPayload = <T extends ClientCredentials>({
  client_auth_method,
  ...values
}: T) =>
  client_auth_method === ClientAuthMethod.Certificate
    ? { ...values, client_secret: '' }
    : { ...values, client_certificate: null, client_private_key: null };

export const validateExternalProviderWizard = async (
  values: AddOpenIdProvider,
): Promise<TestDirectorySyncResponse | boolean | null> => {
  try {
    await api.openIdProvider.addOpenIdProvider(values);
    if (values.directory_sync_enabled) {
      const { data: result } = await api.openIdProvider.testDirectorySync();
      return result;
    }
  } catch (e) {
    const certificateError = clientCertificateError(e);
    if (isPresent(certificateError)) {
      useAddExternalOpenIdStore.setState(({ providerState }) => ({
        activeStep: AddExternalProviderStep.ClientSettings,
        certificateError,
        providerState: {
          ...providerState,
          ...values,
          ...(certificateError.field === 'client_certificate' && {
            client_certificate: null,
          }),
        },
      }));
      return null;
    }
    return false;
  }
  return true;
};

export const directorySyncBehaviorName: Record<DirectorySyncBehaviorValue, string> = {
  delete: m.controls_delete(),
  disable: m.controls_disable(),
  keep: m.controls_keep(),
};

export const directorySyncTargetName: Record<DirectorySyncTargetValue, string> = {
  all: m.cmp_sync_behavior_target_all(),
  groups: m.cmp_sync_behavior_target_groups(),
  users: m.cmp_sync_behavior_target_users(),
};

export const directorySyncBehaviorOptions: SelectOption<DirectorySyncBehaviorValue>[] = [
  {
    key: DirectorySyncBehavior.Keep,
    value: DirectorySyncBehavior.Keep,
    label: directorySyncBehaviorName[DirectorySyncBehavior.Keep],
  },
  {
    label: directorySyncBehaviorName[DirectorySyncBehavior.Disable],
    key: DirectorySyncBehavior.Disable,
    value: DirectorySyncBehavior.Disable,
  },
  {
    key: DirectorySyncBehavior.Delete,
    value: DirectorySyncBehavior.Delete,
    label: directorySyncBehaviorName[DirectorySyncBehavior.Delete],
  },
];

export const directorySyncTargetOptions: SelectOption<DirectorySyncTargetValue>[] = [
  {
    key: DirectorySyncTarget.All,
    value: DirectorySyncTarget.All,
    label: directorySyncTargetName[DirectorySyncTarget.All],
  },
  {
    key: DirectorySyncTarget.Users,
    value: DirectorySyncTarget.Users,
    label: directorySyncTargetName[DirectorySyncTarget.Users],
  },
  {
    key: DirectorySyncTarget.Groups,
    value: DirectorySyncTarget.Groups,
    label: directorySyncTargetName[DirectorySyncTarget.Groups],
  },
];

export const providerUsernameHandlingOptions: SelectOption<OpenIdProviderUsernameHandlingValue>[] =
  [
    {
      key: OpenIdProviderUsernameHandling.RemoveForbidden,
      label: 'Remove forbidden characters',
      value: OpenIdProviderUsernameHandling.RemoveForbidden,
    },
    {
      key: OpenIdProviderUsernameHandling.ReplaceForbidden,
      label: 'Replace forbidden characters',
      value: OpenIdProviderUsernameHandling.ReplaceForbidden,
    },
    {
      key: OpenIdProviderUsernameHandling.PruneEmailDomain,
      label: 'Prune email domain',
      value: OpenIdProviderUsernameHandling.PruneEmailDomain,
    },
  ];

export const jumpcloudRegionOptions: SelectOption<JumpCloudRegion>[] = [
  { key: 'us', value: 'us', label: 'US' },
  { key: 'eu', value: 'eu', label: 'EU' },
  { key: 'in', value: 'in', label: 'India' },
];

export const formatMicrosoftBaseUrl = (tenantId: string) =>
  `https://login.microsoftonline.com/${tenantId}/v2.0`;
