import './style.scss';

import { shallow } from 'zustand/shallow';

import { ModalWithTitle } from '../../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { useUserGroupsListModal } from './useUserGroupsListModal';

export const UserGroupsListModal = () => {
  const isOpen = useUserGroupsListModal((s) => s.visible);
  const [close, reset] = useUserGroupsListModal((s) => [s.close, s.reset], shallow);
  return (
    <ModalWithTitle
      id="user-groups-list-modal"
      isOpen={isOpen}
      onClose={close}
      afterClose={reset}
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const ModalContent = () => {
  const groups = useUserGroupsListModal((s) => s.groups);
  return (
    <div className="scroll-wrapper">
      <div className="groups-list">
        {groups.map((g, i) => (
          <div className="group" key={`${g}-${i}`}>
            <p>{g}</p>
          </div>
        ))}
      </div>
    </div>
  );
};
