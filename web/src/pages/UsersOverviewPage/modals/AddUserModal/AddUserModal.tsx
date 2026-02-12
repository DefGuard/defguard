import { useEffect, useMemo, useRef, useState } from 'react';
import './style.scss';
import { useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import z from 'zod';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import type { User } from '../../../../shared/api/types';
import {
  mapPasswordFieldError,
  refinePasswordField,
} from '../../../../shared/components/modals/ChangePasswordModal/form';
import { SelectionSection } from '../../../../shared/components/SelectionSection/SelectionSection';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Checkbox } from '../../../../shared/defguard-ui/components/Checkbox/Checkbox';
import { CopyField } from '../../../../shared/defguard-ui/components/CopyField/CopyField';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { EvenSplit } from '../../../../shared/defguard-ui/components/EvenSplit/EvenSplit';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SectionSelect } from '../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../../shared/form';
import { formChangeLogic } from '../../../../shared/formLogic';
import { useApp } from '../../../../shared/hooks/useApp';
import {
  patternSafeUsernameCharacters,
  patternValidPhoneNumber,
} from '../../../../shared/patterns';
import { removeEmptyStrings } from '../../../../shared/utils/removeEmptyStrings';
import { useAddUserModal } from './useAddUserModal';

export const AddUserModal = () => {
  const isOpen = useAddUserModal((s) => s.isOpen);
  const step = useAddUserModal((s) => s.step);

  const modalTitle = useMemo(() => {
    switch (step) {
      case 'enroll-choice':
      case 'user':
        return m.modal_add_user_title();
      case 'enrollment':
        return m.modal_add_user_enroll_title();
      case 'groups':
        return m.modal_add_user_groups_title();
    }
  }, [step]);

  return (
    <Modal
      id="add-user-modal"
      title={modalTitle}
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
      {step === 'groups' && <AddUserGroupsSelectionStep />}
      {step === 'enrollment' && <EnrollmentStep />}
    </Modal>
  );
};

const EnrollmentStep = () => {
  const enrollResponse = useAddUserModal((s) => s.enrollResponse);
  const user = useAddUserModal((s) => s.user as User);
  const [sendEmail, setSendEmail] = useState(false);
  const appInfo = useApp((s) => s.appInfo);

  const formSchema = useMemo(
    () =>
      z
        .object({
          email: z.string(),
        })
        .superRefine((values, ctx) => {
          if (sendEmail) {
            const result = z
              .email(m.form_error_email())
              .min(1, m.form_error_required())
              .safeParse(values.email);
            if (!result.success) {
              ctx.addIssue({
                code: 'custom',
                path: ['email'],
                message: result.error.message,
              });
            }
          }
        }),
    [sendEmail],
  );

  const form = useAppForm({
    defaultValues: {
      email: user?.email ?? '',
    },
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      await api.user.startEnrollment({
        username: user.username,
        send_enrollment_notification: true,
        email: value.email,
      });
      useAddUserModal.setState({
        isOpen: false,
      });
    },
  });

  const isSubmitting = useStore(form.store, (s) => s.isSubmitting);

  // biome-ignore lint/correctness/useExhaustiveDependencies: side effect
  useEffect(() => {
    if (!form.state.isPristine) {
      form.validateAllFields('change');
    }
  }, [sendEmail]);

  if (!isPresent(enrollResponse)) return null;

  return (
    <>
      <div className="enrollment-info">
        <AppText font={TextStyle.TBodySm500}>
          {m.modal_add_user_enrollment_details()}
        </AppText>
        <SizedBox height={ThemeSpacing.Xs} />
        <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
          {m.modal_add_user_enrollment_explain()}
        </AppText>
      </div>
      <SizedBox height={ThemeSpacing.Xl2} />
      <CopyField
        copyTooltip={m.misc_clipboard_copy()}
        label={m.modal_add_user_enrollment_form_label_instance_url()}
        data-testid="activation-url-field"
        text={enrollResponse.enrollment_url}
      />
      <SizedBox height={ThemeSpacing.Xl} />
      <CopyField
        label={m.modal_add_user_enrollment_form_label_token()}
        copyTooltip={m.misc_clipboard_copy()}
        data-testid="activation-token-field"
        text={enrollResponse.enrollment_token}
      />
      {appInfo.smtp_enabled && (
        <form
          onSubmit={(e) => {
            e.stopPropagation();
            e.preventDefault();
            form.handleSubmit();
          }}
        >
          <SizedBox height={ThemeSpacing.Xl3} />
          <form.AppForm>
            <Checkbox
              text={m.modal_add_user_enrollment_form_label_send()}
              active={sendEmail}
              onClick={() => {
                setSendEmail((s) => !s);
              }}
            />
            <SizedBox height={ThemeSpacing.Xl} />
            <form.AppField name="email">
              {(field) => (
                <field.FormInput
                  required={sendEmail}
                  disabled={!sendEmail}
                  label={m.form_label_email()}
                />
              )}
            </form.AppField>
          </form.AppForm>
        </form>
      )}
      <ModalControls
        submitProps={{
          text: sendEmail ? m.controls_send_email() : m.controls_complete(),
          loading: isSubmitting,
          onClick: () => {
            if (sendEmail) {
              form.handleSubmit();
            } else {
              useAddUserModal.setState({
                isOpen: false,
              });
            }
          },
        }}
      />
    </>
  );
};

const EnrollmentChoice = () => {
  return (
    <>
      <SectionSelect
        image="self-enrollment"
        title={m.modal_add_user_choice_enroll_title()}
        content={m.modal_add_user_choice_enroll_content()}
        data-testid="add-user-self-enrollment"
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
        data-testid="add-user-manually"
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
  const enrollmentEnabled = useAddUserModal((s) => s.enrollUser);
  const reservedEmails = useAddUserModal((s) => s.reservedEmails);
  const reservedUsernamesStart = useAddUserModal((s) => s.reservedUsernames);
  const reservedUsernames = useRef<string[]>(reservedUsernamesStart);
  const [assignToGroups, setAssignToGroups] = useState(false);

  const { mutateAsync: addUserMutation } = useMutation({
    mutationFn: api.user.addUser,
    meta: {
      invalidate: [['user-overview'], ['user'], ['enterprise_info']],
    },
  });

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
        })
        .superRefine((val, ctx) => {
          // check password
          if (!enrollmentEnabled) {
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
    [reservedEmails, enrollmentEnabled],
  );

  type FormFields = z.infer<typeof formSchema>;

  const defaultValues = useMemo(
    (): FormFields => ({
      email: '',
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
      const { data: created } = await addUserMutation(clean);
      const {
        data: { groups },
      } = await api.group.getGroups();
      if (enrollmentEnabled) {
        try {
          const enrollmentResponse = (
            await api.user.startEnrollment({
              send_enrollment_notification: false,
              username: created.username,
            })
          ).data;
          useAddUserModal.setState({ enrollResponse: enrollmentResponse, user: created });
        } catch (error) {
          console.error(m.failed_to_start_enrollment(), error);
          useAddUserModal.setState({ isOpen: false });
          return;
        }
      }
      if (assignToGroups) {
        useAddUserModal.setState({
          step: 'groups',
          user: created,
          groups,
        });
      } else {
        if (enrollmentEnabled) {
          useAddUserModal.setState({ step: 'enrollment' });
        } else {
          useAddUserModal.setState({ isOpen: false });
        }
      }
    },
  });
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
              {(field) => (
                <field.FormInput
                  data-testid="field-username"
                  required
                  label={m.form_label_username()}
                />
              )}
            </form.AppField>
            <form.AppField name="email">
              {(field) => (
                <field.FormInput
                  data-testid="field-email"
                  required
                  label={m.form_label_email()}
                />
              )}
            </form.AppField>
          </EvenSplit>
          {!enrollmentEnabled && (
            <>
              <SizedBox height={ThemeSpacing.Xl} />
              <form.AppField name="password">
                {(field) => (
                  <field.FormInput
                    required
                    label={m.form_label_password()}
                    mapError={(val) => mapPasswordFieldError(val, true)}
                    type="password"
                  />
                )}
              </form.AppField>
            </>
          )}
          <Divider spacing={ThemeSpacing.Xl} />
          <p>{m.modal_add_user_section_account()}</p>
          <SizedBox height={ThemeSpacing.Lg} />
          <EvenSplit>
            <form.AppField name="first_name">
              {(field) => (
                <field.FormInput
                  data-testid="field-first_name"
                  required
                  label={m.form_label_first_name()}
                />
              )}
            </form.AppField>
            <form.AppField name="last_name">
              {(field) => (
                <field.FormInput
                  data-testid="field-last_name"
                  required
                  label={m.form_label_last_name()}
                />
              )}
            </form.AppField>
          </EvenSplit>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="phone">
            {(field) => (
              <field.FormInput data-testid="field-phone" label={m.form_label_phone()} />
            )}
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
          text: m.modal_add_user_submit(),
          loading: isSubmitting,
          testId: 'add-user-submit',
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

const AddUserGroupsSelectionStep = () => {
  const enrollEnabled = useAddUserModal((s) => s.enrollUser);
  const groups = useAddUserModal((s) => s.groups);
  const user = useAddUserModal((s) => s.user as User);
  const [selected, setSelected] = useState(new Set<string>());

  const { mutate, isPending } = useMutation({
    mutationFn: api.group.addUsersToGroups,
    meta: {
      invalidate: [['group'], ['group-info'], ['user'], ['user-overview']],
    },
    onSuccess: () => {
      if (enrollEnabled) {
        useAddUserModal.setState({
          step: 'enrollment',
        });
      } else {
        useAddUserModal.setState({
          isOpen: false,
        });
      }
    },
  });

  const options = useMemo(
    () =>
      groups.map((group) => ({
        id: group,
        label: group,
      })),
    [groups],
  );

  return (
    <>
      <SelectionSection options={options} selection={selected} onChange={setSelected} />
      <ModalControls
        cancelProps={{
          text: m.controls_close(),
          onClick: () => {
            useAddUserModal.setState({
              isOpen: false,
            });
          },
        }}
        submitProps={{
          text: m.controls_submit(),
          loading: isPending,
          onClick: () => {
            const groups = Array.from(selected);
            if (groups.length) {
              mutate({
                users: [user.id],
                groups: groups,
              });
            } else {
              useAddUserModal.setState({
                isOpen: false,
              });
            }
          },
        }}
      />
    </>
  );
};
