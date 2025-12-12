import z from 'zod';
import { m } from '../../../../../paraglide/messages';
import {
  DirectorySyncBehavior,
  DirectorySyncTarget,
} from '../../../../../shared/api/types';

export const baseExternalProviderSyncSchema = z.object({
  directory_sync_interval: z.number().min(60, m.form_min_value({ value: 60 })),
  directory_sync_user_behavior: z.enum(DirectorySyncBehavior),
  directory_sync_admin_behavior: z.enum(DirectorySyncBehavior),
  directory_sync_target: z.enum(DirectorySyncTarget),
});

export const googleProviderSyncSchema = baseExternalProviderSyncSchema.extend({
  google_service_account_file: z
    .file(m.form_error_required())
    .min(1, m.form_error_required())
    .mime('application/json', m.form_error_file_format()),
  admin_email: z.email(m.form_error_email()).trim().min(1, m.form_error_required()),
});
