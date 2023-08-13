import './style.scss';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { MessageBox } from '../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { ModalWithTitle } from '../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { AddWalletModalForm } from './AddWalletModalForm';

export const AddWalletModal = () => {
  const { LL } = useI18nContext();
  const open = useModalStore((state) => state.addWalletModal.visible);
  const setModalsState = useModalStore((state) => state.setState);

  return (
    <ModalWithTitle
      id="add-wallet-modal"
      title={LL.modals.addWallet.title()}
      isOpen={open}
      setIsOpen={(visibility) =>
        setModalsState({ addWalletModal: { visible: visibility } })
      }
      backdrop
    >
      <MessageBox type={MessageBoxType.INFO} message={LL.modals.addWallet.infoBox()} />
      <AddWalletModalForm />
    </ModalWithTitle>
  );
};
