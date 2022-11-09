import shallow from 'zustand/shallow';

import { ModalWithTitle } from '../../../shared/components/layout/ModalWithTitle/ModalWithTitle';
import { useModalStore } from '../../../shared/hooks/store/useModalStore';

export const LicenseModal = () => {
  const [{ visible: isOpen }, setModalValues] = useModalStore(
    (state) => [state.licenseModal, state.setLicenseModal],
    shallow
  );

  const setIsOpen = (v: boolean) => setModalValues({ visible: v });
  return (
    <ModalWithTitle
      title={`defguard END USER ENTERPRISE LICENSE AGREEMENT`}
      isOpen={isOpen}
      setIsOpen={setIsOpen}
      className="license-modal"
    ></ModalWithTitle>
  );
};
