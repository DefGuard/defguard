import { m } from '../../../../../../../paraglide/messages';
import { Modal } from '../../../../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { useAppForm, withForm } from '../../../../../../../shared/defguard-ui/form';
import './style.scss';
import { useStore } from '@tanstack/react-form';
import clsx from 'clsx';
import { useCallback, useEffect, useMemo, useState } from 'react';
import type z from 'zod';
import type { User } from '../../../../../../../shared/api/types';
import { Icon } from '../../../../../../../shared/defguard-ui/components/Icon';
import type { IconKindValue } from '../../../../../../../shared/defguard-ui/components/Icon/icon-types';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { formChangeLogic } from '../../../../../../../shared/form';
import { useAuth } from '../../../../../../../shared/hooks/useAuth';
import {
  adminChangePasswordDefaultValues,
  adminChangePasswordSchema,
  mapPasswordFieldError,
  type PasswordErrorCodeValue,
  userChangePasswordSchema,
} from './form';

export const ChangePasswordModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [user, setUser] = useState<User | null>(null);
  const isAdmin = useAuth((s) => s.isAdmin);

  return (
    <Modal
      id="change-password-modal"
      title={m.modal_change_password_title()}
      isOpen={isOpen && isPresent(user)}
      onClose={() => setOpen(false)}
      afterClose={() => setUser(null)}
    >
      {isPresent(user) && <ModalContent isAdmin={isAdmin} user={user} />}
    </Modal>
  );
};

const ModalContent = ({ isAdmin, user }: { isAdmin: boolean; user: User }) => {
  const formSchema = useMemo(() => {
    if (isAdmin) {
      return adminChangePasswordSchema;
    }
    return userChangePasswordSchema;
  }, [isAdmin]);

  const form = useAppForm({
    validationLogic: formChangeLogic,
    defaultValues: adminChangePasswordDefaultValues,
    onSubmit: ({ value }) => {
      console.table(value);
      console.log(isAdmin);
    },
    validators: {
      onChange: formSchema,
      onSubmit: formSchema,
    },
  });

  const isSubmitting = useStore(form.store, (s) => s.isSubmitting);

  // biome-ignore lint/correctness/useExhaustiveDependencies: side effect
  useEffect(() => {
    form.reset();
  }, [isAdmin]);

  return (
    <>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          e.stopPropagation();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          {isAdmin && (
            <form.AppField name="password">
              {(field) => (
                <field.FormInput
                  required
                  type="password"
                  label={m.form_label_new_password()}
                  mapError={mapPasswordFieldError}
                />
              )}
            </form.AppField>
          )}
          <CheckList form={form} />
        </form.AppForm>
      </form>
      <ModalControls
        cancelProps={{
          text: m.controls_cancel(),
          disabled: isSubmitting,
        }}
        submitProps={{
          text: m.controls_submit(),
          loading: isSubmitting,
        }}
      />
    </>
  );
};

const CheckList = withForm({
  defaultValues: adminChangePasswordDefaultValues,
  render: ({ form }) => {
    const passwordFieldErrors = useStore(
      form.store,
      (state) =>
        (state.fieldMeta.password?.errors as z.core.$ZodIssue[])
          ?.filter((issue) => issue.code === 'custom')
          .map((issue) => issue.message) ?? [],
    );

    const isPasswordClean = useStore(
      form.store,
      (state) => state.fieldMeta.password?.isPristine ?? true,
    );

    const isChecked = useCallback(
      (value: PasswordErrorCodeValue): boolean =>
        !passwordFieldErrors.includes(value) && !isPasswordClean,
      [passwordFieldErrors, isPasswordClean],
    );

    return (
      <div className="checklist">
        <p>{m.password_form_check_title()}</p>
        <ul>
          <CheckListItem
            errorCode="password_form_check_minimum"
            checked={isChecked('password_form_check_minimum')}
          />
          <CheckListItem
            errorCode="password_form_check_number"
            checked={isChecked('password_form_check_number')}
          />
          <CheckListItem
            errorCode="password_form_check_special"
            checked={isChecked('password_form_check_special')}
          />
          <CheckListItem
            errorCode="password_form_check_lowercase"
            checked={isChecked('password_form_check_lowercase')}
          />
          <CheckListItem
            errorCode="password_form_check_uppercase"
            checked={isChecked('password_form_check_uppercase')}
          />
        </ul>
      </div>
    );
  },
});

const CheckListItem = ({
  checked,
  errorCode,
}: {
  errorCode: PasswordErrorCodeValue;
  checked: boolean;
}) => {
  const iconKind: IconKindValue = checked ? 'check-filled' : 'empty-point';

  return (
    <li
      className={clsx({
        active: checked,
      })}
    >
      <Icon icon={iconKind} size={16} /> <span>{m[errorCode]()}</span>
    </li>
  );
};
