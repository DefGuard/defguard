import { useMemo, useRef, useState } from 'react';
import './style.scss';
import { useStore } from '@tanstack/react-form';
import z from 'zod';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import {
  mapPasswordFieldError,
  refinePasswordField,
} from '../../../../shared/components/modals/ChangePasswordModal/form';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Checkbox } from '../../../../shared/defguard-ui/components/Checkbox/Checkbox';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { EvenSplit } from '../../../../shared/defguard-ui/components/EvenSplit/EvenSplit';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SectionSelect } from '../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { useAppForm } from '../../../../shared/defguard-ui/form';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { formChangeLogic } from '../../../../shared/form';
import {
  patternSafeUsernameCharacters,
  patternValidPhoneNumber,
} from '../../../../shared/patterns';
import { removeEmptyStrings } from '../../../../shared/utils/removeEmptyStrings';
import { useAddUserModal } from './useAddUserModal';

export const AddUserModal = () => {
  const isOpen = useAddUserModal((s) => s.isOpen);
  const step = useAddUserModal((s) => s.step);

  return (
    <Modal
      id="add-user-modal"
      title={m.modal_add_user_title()}
      isOpen={isOpen}
      onClose={() => {
        useAddUserModal.setState({ isOpen: false });
      }}
      afterClose={() => {
        useAddUserModal.getState().reset();
      }}
    >
      {step === 'enroll-choice' && <EnrollmentChoice />}
      {step === 'user' && <AddUserModalForm />}
    </Modal>
  );
};

const EnrollmentChoice = () => {
  return (
    <>
      <SectionSelect
        image="self-enrollment"
        title={m.modal_add_user_choice_enroll_title()}
        content={m.modal_add_user_choice_enroll_content()}
        onClick={() => {
          useAddUserModal.setState({
            step: 'user',
            enrollUser: true,
          });
        }}
      />
      <SizedBox height={ThemeSpacing.Md} />
      <SectionSelect
        image="manual-user"
        title={m.modal_add_user_choice_manual_title()}
        content={m.modal_add_user_choice_manual_content()}
        onClick={() => {
          useAddUserModal.setState({
            step: 'user',
            enrollUser: false,
          });
        }}
      />
    </>
  );
};

const AddUserModalForm = () => {
  const reservedEmails = useAddUserModal((s) => s.reservedEmails);
  const reservedUsernamesStart = useAddUserModal((s) => s.reservedUsernames);
  const reservedUsernames = useRef<string[]>(reservedUsernamesStart);
  const [assignToGroups, setAssignToGroups] = useState(false);

  const formSchema = useMemo(
    () =>
      z
        .object({
          username: z
            .string()
            .trim()
            .min(1, m.form_error_required())
            .max(64, m.form_error_max_len({ length: 64 }))
            .regex(patternSafeUsernameCharacters, m.form_error_forbidden_char()),
          // check in refine
          password: z.string(),
          email: z
            .email()
            .trim()
            .min(1, m.form_error_required())
            .refine((value) => {
              if (isPresent(reservedEmails)) {
                return !reservedEmails.includes(value.toLowerCase());
              }
              return true;
            }, m.form_error_email_reserved()),
          last_name: z.string().trim().min(1, m.form_error_required()),
          first_name: z.string().trim().min(1, m.form_error_required()),
          phone: z.string().trim(),
          enable_enrollment: z.boolean(),
        })
        .superRefine((val, ctx) => {
          // check password
          if (!val.enable_enrollment) {
            const passwordIssues = refinePasswordField(val.password);
            for (const issue of passwordIssues) {
              ctx.addIssue({
                message: issue,
                code: 'custom',
                continue: true,
                path: ['password'],
              });
            }
          }
          if (val.phone?.length) {
            const phoneRes = z
              .string()
              .regex(patternValidPhoneNumber)
              .safeParse(val.phone);
            if (!phoneRes.success) {
              ctx.addIssue({
                code: 'custom',
                path: ['phone'],
                message: m.form_error_invalid(),
              });
            }
          }
          if (reservedUsernames.current.includes(val.username)) {
            ctx.addIssue({
              code: 'custom',
              path: ['username'],
              message: m.form_error_username_taken(),
            });
          }
        }),
    [reservedEmails],
  );

  type FormFields = z.infer<typeof formSchema>;

  const defaultValues = useMemo(
    (): FormFields => ({
      email: '',
      enable_enrollment: false,
      first_name: '',
      last_name: '',
      password: '',
      phone: '',
      username: '',
    }),
    [],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value, formApi }) => {
      let usernameAvailable: boolean;
      try {
        await api.user.usernameAvailable(value.username);
        usernameAvailable = true;
      } catch (_e) {
        usernameAvailable = false;
      }
      if (!usernameAvailable) {
        reservedUsernames.current.push(value.username);
        formApi.validateField('username', 'submit');
        return;
      }
      const clean = removeEmptyStrings(value);
      const { data: created } = await api.user.addUser(clean);
      useAddUserModal.setState({
        step: 'groups',
        user: created,
      });
    },
  });
  const enrollEnabled = useStore(form.store, (s) => s.values.enable_enrollment);
  const isSubmitting = useStore(form.store, (s) => s.isSubmitting);

  return (
    <>
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <p>{m.modal_add_user_section_login()}</p>
          <SizedBox height={ThemeSpacing.Lg} />
          <EvenSplit parts={2}>
            <form.AppField name="username">
              {(field) => <field.FormInput required label={m.form_label_username()} />}
            </form.AppField>
            <form.AppField name="email">
              {(field) => <field.FormInput required label={m.form_label_email()} />}
            </form.AppField>
          </EvenSplit>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="password">
            {(field) => (
              <field.FormInput
                required
                label={m.form_label_password()}
                disabled={enrollEnabled}
                mapError={(val) => mapPasswordFieldError(val, true)}
                type="password"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Md} />
          <form.AppField name="enable_enrollment">
            {(field) => (
              <field.FormCheckbox text={m.modal_add_user_form_enable_enroll_label()} />
            )}
          </form.AppField>
          <Divider spacing={ThemeSpacing.Xl} />
          <p>{m.modal_add_user_section_account()}</p>
          <SizedBox height={ThemeSpacing.Lg} />
          <EvenSplit>
            <form.AppField name="first_name">
              {(field) => <field.FormInput required label={m.form_label_first_name()} />}
            </form.AppField>
            <form.AppField name="last_name">
              {(field) => <field.FormInput required label={m.form_label_last_name()} />}
            </form.AppField>
          </EvenSplit>
          <Divider spacing={ThemeSpacing.Xl} />
          <form.AppField name="phone">
            {(field) => <field.FormInput label={m.form_label_phone()} />}
          </form.AppField>
        </form.AppForm>
      </form>
      <SizedBox height={ThemeSpacing.Xl2} />
      <Checkbox
        active={assignToGroups}
        text={m.modal_add_user_assign_groups_checkbox()}
        onClick={() => {
          setAssignToGroups((s) => !s);
        }}
      />
      <ModalControls
        cancelProps={{
          disabled: isSubmitting,
          text: m.controls_cancel(),
          onClick: () => {},
        }}
        submitProps={{
          text: m.controls_next(),
          loading: isSubmitting,
          onClick: () => {
            form.handleSubmit();
          },
        }}
      >
        <Button
          variant="outlined"
          onClick={() => {
            useAddUserModal.setState({
              step: 'enroll-choice',
            });
          }}
          text={m.controls_back()}
        />
      </ModalControls>
    </>
  );
};
