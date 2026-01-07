import z from 'zod';
import { m } from '../../paraglide/messages';

export const totpCodeFormSchema = z.object({
  code: z
    .string()
    .trim()
    .min(1, m.form_error_required())
    .min(
      6,
      m.form_error_min_len({
        length: 6,
      }),
    )
    .max(6, m.form_error_max_len({ length: 6 })),
});
