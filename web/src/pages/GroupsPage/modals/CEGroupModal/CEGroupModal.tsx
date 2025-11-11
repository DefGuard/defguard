import z from 'zod';
import { m } from '../../../../paraglide/messages';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import type { OpenCEGroupModal } from '../../../../shared/hooks/modalControls/types';
import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { type Dispatch, type SetStateAction, useEffect, useMemo, useState } from 'react';
import api from '../../../../shared/api/api';
import type { CreateGroupRequest, User } from '../../../../shared/api/types';
import { SelectionSection } from '../../../../shared/components/SelectionSection/SelectionSection';
import type { SelectionSectionOption } from '../../../../shared/components/SelectionSection/type';
import { Checkbox } from '../../../../shared/defguard-ui/components/Checkbox/Checkbox';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { useAppForm } from '../../../../shared/defguard-ui/form';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { formChangeLogic } from '../../../../shared/form';

interface ModalState extends OpenCEGroupModal {
  step: 'start' | 'users';
  startForm?: Pick<CreateGroupRequest, 'is_admin' | 'name'>;
}

const modalNameKey = ModalName.CreateEditGroup;

export const CEGroupModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalState, setModalState] = useState<ModalState | null>(null);

  const isEdit = useMemo(() => isPresent(modalState?.groupInfo), [modalState]);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameKey, (data) => {
      // assign first step data on open bcs you can "back" into it from users selection the edit state needs to be assigned once on open otherwise after back the form will be reset every time
      const startForm: ModalState['startForm'] = isPresent(data.groupInfo)
        ? {
            is_admin: data.groupInfo.is_admin,
            name: data.groupInfo.name,
          }
        : undefined;
      setModalState({
        ...data,
        startForm: startForm,
        step: 'start',
      });
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameKey, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal
      id="ce-group-modal"
      title={isEdit ? m.modal_edit_group_title() : m.modal_add_group_title_start()}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {
        setModalState(null);
      }}
    >
      {modalState && modalState.step === 'start' && (
        <StartStep {...modalState} setModalState={setModalState} isEdit={isEdit} />
      )}
      {modalState && modalState.step === 'users' && (
        <UsersStep {...modalState} setModalState={setModalState} isEdit={isEdit} />
      )}
    </Modal>
  );
};

interface StepProps extends ModalState {
  isEdit: boolean;
  setModalState: Dispatch<SetStateAction<ModalState | null>>;
}

const userToOption = (user: User): SelectionSectionOption<string> => ({
  id: user.username,
  label: `${user.first_name} ${user.last_name}`,
});

const UsersStep = ({ users, startForm, groupInfo, isEdit, setModalState }: StepProps) => {
  const { mutate: editGroup, isPending: editPending } = useMutation({
    mutationFn: api.group.editGroup,
    meta: {
      invalidate: [['group'], ['group-info'], ['user']],
    },
    onSuccess: () => {
      closeModal(modalNameKey);
    },
  });

  const { mutate: addGroup, isPending: addPending } = useMutation({
    mutationFn: api.group.addGroup,
    meta: {
      invalidate: [['group'], ['group-info'], ['user']],
    },
    onSuccess: () => {
      closeModal(modalNameKey);
    },
  });
  const [selected, setSelected] = useState<Set<string>>(
    new Set(groupInfo?.members ?? []),
  );

  const options = useMemo(() => users.map((user) => userToOption(user)), [users]);

  const handleSubmit = () => {
    if (startForm && !addPending && !editPending) {
      const members = Array.from(selected);
      const requestData = {
        ...startForm,
        members: members,
      };
      if (isEdit) {
        editGroup(requestData);
      } else {
        addGroup(requestData);
      }
    }
  };

  return (
    <>
      <SelectionSection options={options} selection={selected} onChange={setSelected} />
      <ModalControls
        cancelProps={{
          disabled: addPending || editPending,
          text: m.controls_back(),
          onClick: () => {
            setModalState((s) => {
              if (s) {
                return { ...s, step: 'start' };
              }
              return null;
            });
          },
        }}
        submitProps={{
          text: isEdit ? m.controls_save_changes() : m.controls_submit(),
          loading: addPending || editPending,
          onClick: () => {
            handleSubmit();
          },
        }}
      />
    </>
  );
};

const StartStep = ({ reservedNames, setModalState, groupInfo, startForm }: StepProps) => {
  const [isAdmin, setIsAdmin] = useState(startForm?.is_admin ?? false);

  const formSchema = useMemo(
    () =>
      z.object({
        name: z
          .string()
          .trim()
          .min(1, m.form_error_required())
          .refine((value) => {
            // exclude initial name
            if (groupInfo && groupInfo.name === value) return true;
            return !reservedNames.includes(value);
          }, m.form_error_name_reserved()),
      }),
    [reservedNames, groupInfo],
  );

  const form = useAppForm({
    defaultValues: {
      name: startForm?.name ?? '',
    },
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: ({ value }) => {
      setModalState((s) => {
        if (isPresent(s)) {
          return {
            ...s,
            step: 'users',
            startForm: {
              name: value.name,
              is_admin: isAdmin,
            },
          };
        }
        return null;
      });
    },
  });

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
          <form.AppField name="name">
            {(field) => (
              <field.FormInput label={m.modal_add_group_form_label_name()} required />
            )}
          </form.AppField>
        </form.AppForm>
        <Divider spacing={ThemeSpacing.Xl} />
        <p>{m.modal_add_group_admin_title()}</p>
        <p>{m.modal_add_group_admin_explain()}</p>
        <Divider spacing={ThemeSpacing.Lg} />
        <Checkbox
          text={m.modal_add_group_form_label_admin()}
          active={isAdmin}
          onClick={() => {
            setIsAdmin((s) => !s);
          }}
        />
      </form>
      <ModalControls
        cancelProps={{
          text: m.controls_cancel(),
          onClick: () => {
            closeModal(modalNameKey);
          },
        }}
        submitProps={{
          text: m.controls_next(),
          onClick: () => {
            form.handleSubmit();
          },
        }}
      />
    </>
  );
};
