import { useMutation } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { SelectionSection } from '../../../../shared/components/SelectionSection/SelectionSection';
import type { SelectionOption } from '../../../../shared/components/SelectionSection/type';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import type { OpenAssignUsersToGroupsModal } from '../../../../shared/hooks/modalControls/types';

const modalNameValue = ModalName.AssignGroupsToUsers;

type ModalData = OpenAssignUsersToGroupsModal;

export const AssignUsersToGroupsModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameValue, (data) => {
      setModalData(data);
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameValue, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal
      title={m.modal_assign_users_groups_title()}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {
        setModalData(null);
      }}
    >
      {isPresent(modalData) && <ModalContent {...modalData} />}
    </Modal>
  );
};

const ModalContent = ({ groups, users }: ModalData) => {
  const [selection, setSelection] = useState<Set<string>>(new Set());

  const { mutate, isPending } = useMutation({
    mutationFn: api.group.addUsersToGroups,
    onSuccess: () => {
      closeModal(modalNameValue);
    },
    meta: {
      invalidate: [['user-overview'], ['user']],
    },
  });

  const options = useMemo(
    (): SelectionOption<string>[] =>
      groups.map((g) => ({
        id: g.name,
        label: g.name,
      })),
    [groups],
  );

  return (
    <>
      <SelectionSection onChange={setSelection} selection={selection} options={options} />
      <ModalControls
        cancelProps={{
          text: m.controls_cancel(),
          disabled: isPending,
          onClick: () => {
            closeModal(modalNameValue);
          },
        }}
        submitProps={{
          text: m.controls_submit(),
          testId: 'submit',
          loading: isPending,
          onClick: () => {
            const selected = Array.from(selection);
            mutate({
              groups: selected,
              users: users,
            });
          },
        }}
      />
    </>
  );
};
