import './style.scss';

import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { ModalWithTitle } from '../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { AddApiTokenForm } from './components/AddApiTokenForm/AddApiTokenForm';
import { useAddApiTokenModal } from './useAddApiTokenModal';

export const AddApiTokenModal = () => {
  const { LL } = useI18nContext();

  const [close, reset] = useAddApiTokenModal((s) => [s.close, s.reset], shallow);

  const isProvisioning = useAddApiTokenModal((s) => s.provisioningInProgress);

  const isOpen = useAddApiTokenModal((s) => s.visible);

  return (
    <ModalWithTitle
      id="add-api-token-modal"
      backdrop
      title={LL.userPage.apiTokens.addModal.header()}
      onClose={close}
      afterClose={reset}
      isOpen={isOpen}
      disableClose={isProvisioning}
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const ModalContent = () => {
  return (
    <>
      <AddApiTokenForm />
    </>
  );
};
