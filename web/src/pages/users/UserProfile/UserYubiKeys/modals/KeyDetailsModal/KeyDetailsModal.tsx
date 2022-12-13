import './style.scss';

import { saveAs } from 'file-saver';
import { motion } from 'framer-motion';
import React from 'react';
import shallow from 'zustand/shallow';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/components/layout/Button/Button';
import Modal from '../../../../../../shared/components/layout/Modal/Modal';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import KeyBox from '../../../../shared/components/KeyBox/KeyBox';

const KeyDetailsModal: React.FC = () => {
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
        <p>YubiKey details</p>
        {/* <Button className="small warning">
            <span>Delete YubiKey</span>
          </Button> */}
      </div>
      {user ? (
        <motion.ul layout className="keys-list">
          {user.pgp_key && user.pgp_key !== '-' ? (
            <motion.li layout>
              <KeyBox collapsible keyValue={user.pgp_key} title="PGP key" />
            </motion.li>
          ) : null}
          {user.ssh_key && user.ssh_key !== '-' ? (
            <motion.li layout>
              <KeyBox collapsible keyValue={user.ssh_key} title="SSH key" />
            </motion.li>
          ) : null}
        </motion.ul>
      ) : null}
      <div className="controls">
        <Button
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.STANDARD}
          className="cancel"
          text="Close"
          onClick={() => setIsOpen(false)}
        />
        <Button
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.PRIMARY}
          className="big primary"
          onClick={() => handleDownloadAll()}
          text="Download all keys"
        />
      </div>
    </Modal>
  );
};

export default KeyDetailsModal;
