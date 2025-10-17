import './style.scss';
import { revalidateLogic, useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../../../../../paraglide/messages';
import api from '../../../../../../../shared/api/api';
import { FormRow } from '../../../../../../../shared/components/FormRow/FormRow';
import { Button } from '../../../../../../../shared/defguard-ui/components/Button/Button';
import { useAppForm } from '../../../../../../../shared/defguard-ui/form';
import {
  patternSafeUsernameCharacters,
  patternValidEmail,
  patternValidPhoneNumber,
} from '../../../../../../../shared/patterns';
import { ProfileCard } from '../../../../components/ProfileCard/ProfileCard';
import { useUserProfile } from '../../../../hooks/useUserProfilePage';

const zodSchema = z.object({
  username: z
    .string()
    .trim()
    .min(1, m.form_error_required())
    .regex(patternSafeUsernameCharacters, m.form_error_forbidden_char())
    .max(
      64,
      m.form_error_max_len({
        length: 64,
      }),
    ),
  first_name: z.string().trim().min(1, m.form_error_required()),
  last_name: z.string().trim().min(1, m.form_error_required()),
  phone: z
    .string()
    .trim()
    .optional()
    .refine((val) => {
      if (val?.length) {
        return patternValidPhoneNumber.test(val);
      }
      return true;
    }, m.form_error_invalid()),
  email: z
    .string()
    .trim()
    .min(1, m.form_error_required())
    .regex(patternValidEmail, m.form_error_invalid()),
  groups: z.array(z.string().trim().min(1, m.form_error_required())),
  authorized_apps: z.array(
    z.object({
      oauth2client_id: z.number().min(1, m.form_error_required()),
      oauth2client_name: z.string().trim().min(1, m.form_error_required()),
      user_id: z.number().min(1, m.form_error_required()),
    }),
  ),
  is_active: z.boolean(),
});

type FormFields = z.infer<typeof zodSchema>;

export const ProfileGeneralCard = () => {
  const profileUser = useUserProfile((s) => s.profile.user);
  const isAdmin = profileUser.is_admin;

  const { mutateAsync } = useMutation({
    mutationFn: api.user.editUser,
    meta: {
      invalidate: [['user', profileUser.username]],
    },
  });

  const defaultValues = useUserProfile(
    useShallow(
      (s): FormFields => ({
        authorized_apps: s.profile.user.authorized_apps ?? [],
        email: s.profile.user.email,
        first_name: s.profile.user.first_name,
        last_name: s.profile.user.last_name,
        groups: s.profile.user.groups,
        is_active: s.profile.user.is_active,
        username: s.profile.user.username,
      }),
    ),
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: revalidateLogic({
      mode: 'change',
      modeAfterSubmission: 'change',
    }),
    onSubmit: async ({ value }) => {
      await mutateAsync({
        username: profileUser.username,
        body: { ...profileUser, ...value },
      });
    },
  });

  const isPristine = useStore(form.store, (s) => s.isDefaultValue || s.isPristine);
  const isSubmitting = useStore(form.store, (s) => s.isSubmitting);

  const fieldProps = {
    disabled: !isAdmin,
    required: isAdmin,
  };

  return (
    <ProfileCard id="general-card">
      <h2>General</h2>
      <form>
        <form.AppForm>
          <form.AppField name="username">
            {(field) => (
              <field.FormInput {...fieldProps} label={m.form_label_username()} />
            )}
          </form.AppField>
          <FormRow>
            <form.AppField name="first_name">
              {(field) => (
                <field.FormInput {...fieldProps} label={m.form_label_first_name()} />
              )}
            </form.AppField>
            <form.AppField name="last_name">
              {(field) => (
                <field.FormInput {...fieldProps} label={m.form_label_last_name()} />
              )}
            </form.AppField>
          </FormRow>
          <form.AppField name="phone">
            {(field) => <field.FormInput label={m.form_label_phone()} />}
          </form.AppField>
          <form.AppField name="email">
            {(field) => <field.FormInput {...fieldProps} label={m.form_label_email()} />}
          </form.AppField>
          <Button
            type="submit"
            variant="primary"
            text={m.controls_save_changes()}
            disabled={isPristine}
            loading={isSubmitting}
          />
        </form.AppForm>
      </form>
    </ProfileCard>
  );
};
