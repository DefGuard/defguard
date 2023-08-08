import './style.scss';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { AddWalletModalForm } from './AddWalletModalForm';
import { MessageBox } from '../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { ModalWithTitle } from '../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';

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
      <MessageBox type={MessageBoxType.INFO}>
        <p>{LL.modals.addWallet.infoBox()}</p>
      </MessageBox>
      <AddWalletModalForm />
    </ModalWithTitle>
  );
};
