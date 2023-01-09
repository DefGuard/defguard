import './style.scss';

import { saveAs } from 'file-saver';
import { motion } from 'framer-motion';
import shallow from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/components/layout/Button/Button';
import Modal from '../../../../../../shared/components/layout/Modal/Modal';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import KeyBox from '../../../../shared/components/KeyBox/KeyBox';

export const KeyDetailsModal = () => {
  const { LL } = useI18nContext();
  const [{ visible: isOpen, user }, setModalValues] = useModalStore(
    (state) => [state.keyDetailModal, state.setKeyDetailModal],
    shallow
  );

  const setIsOpen = (v: boolean) => setModalValues({ visible: v });

  if (!isOpen) return null;

  const handleDownloadAll = () => {
    if (user && user.pgp_key && user.ssh_key) {
      const pgp = new Blob([user.pgp_key], {
        type: 'text/plain;charset=utf-8',
      });
      const ssh = new Blob([user.ssh_key], {
        type: 'text/plain;charset=utf-8',
      });
      saveAs(pgp, `pgp_key.txt`);
      saveAs(ssh, 'ssh_key.txt');
    }
  };

  return (
    <Modal
      backdrop
      setIsOpen={setIsOpen}
      className="key-details middle"
      isOpen={isOpen}
    >
      <div className="header">
        <p>{LL.modals.keyDetails.title()}</p>
      </div>
      {user ? (
        <motion.ul layout className="keys-list">
          {user.pgp_key && user.pgp_key !== '-' ? (
            <motion.li layout>
              <KeyBox
                collapsible
                keyValue={user.pgp_key}
                title={LL.userPage.yubiKey.keys.pgp()}
              />
            </motion.li>
          ) : null}
          {user.ssh_key && user.ssh_key !== '-' ? (
            <motion.li layout>
              <KeyBox
                collapsible
                keyValue={user.ssh_key}
                title={LL.userPage.yubiKey.keys.ssh()}
              />
            </motion.li>
          ) : null}
        </motion.ul>
      ) : null}
      <div className="controls">
        <Button
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.STANDARD}
          className="cancel"
          text={LL.form.cancel()}
          onClick={() => setIsOpen(false)}
        />
        <Button
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          className="big primary"
          onClick={() => handleDownloadAll()}
          text={LL.modals.keyDetails.downloadAll()}
        />
      </div>
    </Modal>
  );
};
