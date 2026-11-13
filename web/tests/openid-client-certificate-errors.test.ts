import { describe, expect, it } from 'vitest';
import z from 'zod';
import {
  ClientAuthMethod,
  clientCertificateError,
  clientCredentialsPayload,
  clientCredentialsSchema,
  refineClientCertificate,
} from '../src/pages/AddExternalOpenIdWizardPage/consts';
import { m } from '../src/paraglide/messages';

const apiError = (code: string) => ({ response: { data: { code, msg: 'msg' } } });

const missingPaths = (certificate: string | null, key: string | null) =>
  clientCredentialsSchema
    .superRefine(refineClientCertificate)
    .safeParse({
      client_auth_method: ClientAuthMethod.Certificate,
      client_certificate: certificate,
      client_private_key: key,
    })
    .error?.issues.map((issue) => issue.path.join('.'));

describe('clientCertificateError', () => {
  it('puts certificate problems under the certificate and key problems under the key', () => {
    expect(clientCertificateError(apiError('cert_expired'))).toEqual({
      field: 'client_certificate',
      message: m.api_error_cert_expired(),
    });
    expect(clientCertificateError(apiError('cert_parse_error'))?.field).toBe(
      'client_certificate',
    );
    expect(clientCertificateError(apiError('cert_missing_key_pem'))?.field).toBe(
      'client_private_key',
    );
    expect(clientCertificateError(apiError('cert_invalid_cert_or_key'))?.field).toBe(
      'client_private_key',
    );
  });

  it('ignores errors that are not about the certificate', () => {
    expect(clientCertificateError(apiError('bad_request'))).toBeNull();
    expect(clientCertificateError(apiError('cert_unknown'))).toBeNull();
    expect(clientCertificateError(new Error('network'))).toBeNull();
  });
});

describe('clientCredentialsPayload', () => {
  const values = {
    client_id: 'id',
    client_secret: 'secret',
    client_certificate: 'cert',
    client_private_key: 'key',
  };

  it('sends only the credentials of the selected method, without the method itself', () => {
    expect(
      clientCredentialsPayload({
        ...values,
        client_auth_method: ClientAuthMethod.Certificate,
      }),
    ).toEqual({ ...values, client_secret: '' });
    expect(
      clientCredentialsPayload({
        ...values,
        client_auth_method: ClientAuthMethod.Secret,
      }),
    ).toEqual({ ...values, client_certificate: null, client_private_key: null });
  });
});

describe('refineClientCertificate', () => {
  it('shows one missing-files message, under the key when both files are missing', () => {
    expect(missingPaths(null, null)).toEqual(['client_private_key']);
    expect(missingPaths(null, 'key')).toEqual(['client_certificate']);
    expect(missingPaths('cert', null)).toEqual(['client_private_key']);
    expect(missingPaths('cert', 'key')).toBeUndefined();
  });

  it('accepts no files when the client secret method is selected', () => {
    const result = z
      .object(clientCredentialsSchema.shape)
      .superRefine(refineClientCertificate)
      .safeParse({
        client_auth_method: ClientAuthMethod.Secret,
        client_certificate: null,
        client_private_key: null,
      });
    expect(result.success).toBe(true);
  });
});
