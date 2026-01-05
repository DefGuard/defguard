import z from 'zod';
import { m } from '../../../../../paraglide/messages';
import {
  DirectorySyncBehavior,
  DirectorySyncTarget,
  OpenIdProviderUsernameHandling,
} from '../../../../../shared/api/types';
import { isPresent } from '../../../../../shared/defguard-ui/utils/isPresent';

export const baseExternalProviderConfigSchema = z.object({
  base_url: z.url(m.form_error_invalid()).trim().min(1, m.form_error_required()),
  client_id: z.string(m.form_error_required()).trim().min(1, m.form_error_required()),
  client_secret: z.string(m.form_error_required()).trim().min(1, m.form_error_required()),
  display_name: z.string(m.form_error_required()).trim().min(1, m.form_error_required()),
  create_account: z.boolean(m.form_error_invalid()),
  username_handling: z.enum(OpenIdProviderUsernameHandling),
});

export const baseExternalProviderSyncSchema = z.object({
  directory_sync_interval: z.number().min(60, m.form_min_value({ value: 60 })),
  directory_sync_user_behavior: z.enum(DirectorySyncBehavior),
  directory_sync_admin_behavior: z.enum(DirectorySyncBehavior),
  directory_sync_target: z.enum(DirectorySyncTarget),
});

export const googleProviderSyncSchema = baseExternalProviderSyncSchema.extend({
  admin_email: z.email(m.form_error_email()).trim().min(1, m.form_error_required()),
  google_service_account_file: z
    .file(m.form_error_required())
    .mime('application/json', m.form_error_file_format())
    .nullable(),
});

export const microsoftProviderSyncSchema = baseExternalProviderSyncSchema.extend({
  prefetch_users: z.boolean(),
  directory_sync_group_match: z.string().trim().nullable(),
});

export const oktaProviderSyncSchema = baseExternalProviderSyncSchema.extend({
  okta_dirsync_client_id: z
    .string(m.form_error_required())
    .trim()
    .min(1, m.form_error_required(0)),
  okta_private_jwk: z
    .string(m.form_error_required())
    .trim()
    .min(1, m.form_error_required()),
});

export const jumpcloudProviderSyncSchema = baseExternalProviderSyncSchema.extend({
  jumpcloud_api_key: z
    .string(m.form_error_required())
    .trim()
    .min(1, m.form_error_required()),
});

const fileSchema = z.object({
  private_key: z.string().trim().min(1),
  client_email: z.string().trim().min(1),
});

type GoogleAccountFileObject = z.infer<typeof fileSchema>;

export const parseGoogleKeyFile = async (
  value: File,
): Promise<GoogleAccountFileObject | null> => {
  try {
    const obj = JSON.parse(await value.text());
    const result = fileSchema.safeParse(obj);
    if (result.success) {
      return result.data;
    }
  } catch (_) {
    return null;
  }
  return null;
};

export const providerToGoogleKeyFile = (
  key?: string | null,
  email?: string | null,
): File | null => {
  if (!isPresent(key) || !isPresent(email)) return null;

  const obj: GoogleAccountFileObject = {
    client_email: email,
    private_key: key,
  };
  return new File([JSON.stringify(obj)], 'Account key', { type: 'application/json' });
};
