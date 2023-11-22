import './style.scss';

import prase from 'html-react-parser';
import { useMemo } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { MessageBox } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { ModalWithTitle } from '../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { useUserProfileStore } from '../../../../../../shared/hooks/store/useUserProfileStore';
import { RegisterMFAEmailForm } from './components/RegisterMFAEmailForm/RegisterMFAEmailForm';
import { useEmailMFAModal } from './hooks/useEmailMFAModal';

export const RegisterEmailMFAModal = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.registerEmailMFA;
  const visible = useEmailMFAModal((state) => state.visible);
  const [close, reset] = useEmailMFAModal((state) => [state.close, state.reset], shallow);

  return (
    <ModalWithTitle
      title={localLL.title()}
      isOpen={visible}
      onClose={close}
      afterClose={reset}
      id="register-mfa-email-modal"
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const ModalContent = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.registerEmailMFA;
  const userProfile = useUserProfileStore((state) => state.userProfile);
  const infoMessage = useMemo(
    () => prase(localLL.infoMessage({ email: userProfile?.user.email ?? '' })),
    [localLL, userProfile?.user.email],
  );
  return (
    <>
      <MessageBox type={MessageBoxType.INFO} message={infoMessage} />
      <RegisterMFAEmailForm />
    </>
  );
};
