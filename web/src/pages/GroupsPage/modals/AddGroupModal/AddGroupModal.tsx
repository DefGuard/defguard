import z from 'zod';
import { m } from '../../../../paraglide/messages';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import type { OpenAddGroupModal } from '../../../../shared/hooks/modalControls/types';
import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { type Dispatch, type SetStateAction, useEffect, useMemo, useState } from 'react';
import api from '../../../../shared/api/api';
import type { CreateGroupRequest, GroupInfo, User } from '../../../../shared/api/types';
import { Checkbox } from '../../../../shared/defguard-ui/components/Checkbox/Checkbox';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SelectionSection } from '../../../../shared/defguard-ui/components/SelectionSection/SelectionSection';
import type { SelectionSectionOption } from '../../../../shared/defguard-ui/components/SelectionSection/type';
import { useAppForm } from '../../../../shared/defguard-ui/form';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { formChangeLogic } from '../../../../shared/form';

interface ModalState extends OpenAddGroupModal {
  step: 'start' | 'users';
  initialState?: GroupInfo;
  startForm?: Pick<CreateGroupRequest, 'is_admin' | 'name'>;
}

const modalNameKey = ModalName.AddGroup;

export const AddGroupModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalState, setModalState] = useState<ModalState | null>(null);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameKey, (data) => {
      setModalState({
        ...data,
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
      title={m.modal_add_group_title_start()}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {
        setModalState(null);
      }}
    >
      {modalState && modalState.step === 'start' && (
        <StartStep {...modalState} setModalState={setModalState} />
      )}
      {modalState && modalState.step === 'users' && (
        <UsersStep {...modalState} setModalState={setModalState} />
      )}
    </Modal>
  );
};

interface StepProps extends ModalState {
  setModalState: Dispatch<SetStateAction<ModalState | null>>;
}

const userToOption = (user: User): SelectionSectionOption<string> => ({
  id: user.username,
  label: `${user.first_name} ${user.last_name}`,
});

const UsersStep = ({ users, startForm, initialState, setModalState }: StepProps) => {
  const { mutate, isPending } = useMutation({
    mutationFn: api.group.addGroup,
    meta: {
      invalidate: [['group'], ['group-info']],
    },
    onSuccess: () => {
      closeModal(modalNameKey);
    },
  });
  const [selected, setSelected] = useState<Set<string>>(
    new Set(initialState?.members ?? []),
  );

  const options = useMemo(() => users.map((user) => userToOption(user)), [users]);

  const handleSubmit = () => {
    if (startForm && !isPending) {
      const members = Array.from(selected);
      mutate({
        ...startForm,
        members: members,
      });
    }
  };

  return (
    <>
      <SelectionSection
        options={options}
        selection={selected}
        onChange={setSelected}
        searchPlaceholder={m.cmp_selection_section_search_placeholder()}
        selectAllText={m.modal_add_user_groups_select_all()}
        selectedOnlyText={m.cmp_selection_section_selected_filter()}
      />
      <ModalControls
        cancelProps={{
          disabled: isPending,
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
          text: m.controls_submit(),
          loading: isPending,
          onClick: () => {
            handleSubmit();
          },
        }}
      />
    </>
  );
};

const StartStep = ({ reservedNames, setModalState }: StepProps) => {
  const [isAdmin, setIsAdmin] = useState(false);

  const formSchema = useMemo(
    () =>
      z.object({
        name: z
          .string()
          .trim()
          .min(1, m.form_error_required())
          .refine(
            (value) => !reservedNames.includes(value),
            m.form_error_name_reserved(),
          ),
      }),
    [reservedNames],
  );

  const form = useAppForm({
    defaultValues: {
      name: '',
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
