import './style.scss';

import clipboard from 'clipboardy';
import { saveAs } from 'file-saver';
import ReactMarkdown from 'react-markdown';
import { shallow } from 'zustand/shallow';

import licenseAgreement from '../../../shared/assets/LICENSE.md?raw';
import Button, {
  ButtonStyleVariant,
} from '../../../shared/components/layout/Button/Button';
import Modal from '../../../shared/components/layout/Modal/Modal';
import { IconCancel, IconCopy, IconDownload } from '../../../shared/components/svg';
import { useModalStore } from '../../../shared/hooks/store/useModalStore';
import { useToaster } from '../../../shared/hooks/useToaster';

export const LicenseModal = () => {
  const toaster = useToaster();
  const [{ visible: isOpen }, setModalValues] = useModalStore(
    (state) => [state.licenseModal, state.setLicenseModal],
    shallow
  );
  const setIsOpen = (v: boolean) => setModalValues({ visible: v });

  const handleDownload = () => {
    const blob = new Blob([licenseAgreement], {
      type: 'text/plain;charset=utf-8',
    });
    saveAs(blob, `Defguard-License.txt`);
  };

  return (
    <Modal
      backdrop
      isOpen={isOpen}
      setIsOpen={setIsOpen}
      className="license-modal middle"
    >
      <div className="header">
        <IconCancel onClick={() => setIsOpen(false)} />
        <h1>defguard</h1>
        <h2>END USER ENTERPRISE LICENSE AGREEMENT</h2>
      </div>
      <div className="license-text">
        <ReactMarkdown>{licenseAgreement}</ReactMarkdown>
      </div>
      <div className="controls">
        <Button
          styleVariant={ButtonStyleVariant.STANDARD}
          icon={<IconCopy />}
          onClick={() => {
            if (licenseAgreement) {
              clipboard
                .write(licenseAgreement)
                .then(() => {
                  toaster.success('License copied');
                })
                .catch((err) => {
                  console.error(err);
                  toaster.error('Clipboard unaccessable');
                });
            }
          }}
        />
        <Button
          onClick={handleDownload}
          styleVariant={ButtonStyleVariant.STANDARD}
          icon={<IconDownload />}
        />
      </div>
    </Modal>
  );
};
