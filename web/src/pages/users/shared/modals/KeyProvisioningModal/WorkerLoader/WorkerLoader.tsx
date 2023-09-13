import './style.scss';

import { saveAs } from 'file-saver';
import { AnimatePresence, motion } from 'framer-motion';
import React from 'react';

import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { LoaderSpinner } from '../../../../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { MessageBox } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { useModalStore } from '../../../../../../shared/hooks/store/useModalStore';
import { WorkerJobStatus } from '../../../../../../shared/types';
import { KeyBox } from '../../../components/KeyBox/KeyBox';

interface Props {
  setIsOpen: (v: boolean) => void;
  succeeded: boolean;
  errorData: string;
  keyData?: Pick<WorkerJobStatus, 'pgp_cert_id' | 'pgp_key' | 'ssh_key'>;
}

const WorkerLoader: React.FC<Props> = ({ setIsOpen, succeeded, keyData, errorData }) => {
  const user = useModalStore((state) => state.provisionKeyModal.user);

  const handleDownloadAll = () => {
    if (keyData && keyData.pgp_key && keyData.ssh_key) {
      const pgp = new Blob([keyData.pgp_key], {
        type: 'text/plain;charset=utf-8',
      });
      const ssh = new Blob([keyData.ssh_key], {
        type: 'text/plain;charset=utf-8',
      });
      saveAs(pgp, `pgp_key.txt`);
      saveAs(ssh, 'ssh_key.txt');
    }
  };

  return (
    <AnimatePresence mode="wait">
      {!succeeded ? (
        <motion.div
          initial={{
            left: 0,
            top: 0,
            opacity: 1,
          }}
          animate={{
            left: 0,
            opacity: 1,
            height: '100%',
          }}
          exit={{
            top: 20,
            opacity: 0,
            transition: {
              duration: 0.3,
            },
          }}
          className="worker-loader"
          key="worker-loader-loading"
        >
          <LoaderSpinner size={108} />
          <p className="title">Yubikey is being provisioned</p>
          <p className="sub-title">Please wait</p>
        </motion.div>
      ) : null}
      {succeeded && !errorData.length ? (
        <motion.div
          initial={{ top: -20, left: 0, opacity: 0 }}
          animate={{ top: 0, opacity: 1 }}
          transition={{ duration: 0.3 }}
          className="worker-loader"
          key="worker-loader-success"
        >
          <p className="title">Success!</p>
          <p className="sub-title">
            Yubikey provisioning for {`${user?.first_name} ${user?.last_name}`} has been
            completed.
          </p>
          {keyData && keyData.pgp_key && keyData.ssh_key ? (
            <ul>
              <li>
                <KeyBox collapsible keyValue={keyData?.pgp_key} title="PGP key" />
              </li>
              <li>
                <KeyBox collapsible keyValue={keyData?.ssh_key} title="SSH key" />
              </li>
            </ul>
          ) : null}
          <div className="controls">
            <Button
              size={ButtonSize.LARGE}
              styleVariant={ButtonStyleVariant.STANDARD}
              text="Close"
              className="cancel"
              onClick={() => setIsOpen(false)}
            />
            <Button
              size={ButtonSize.LARGE}
              styleVariant={ButtonStyleVariant.PRIMARY}
              text="Download all keys"
              onClick={handleDownloadAll}
            />
          </div>
        </motion.div>
      ) : null}
      {succeeded && errorData.length ? (
        <motion.div
          initial={{ top: -20, left: 0, opacity: 0 }}
          animate={{ top: 0, opacity: 1 }}
          transition={{ duration: 0.3 }}
          className="worker-loader"
          key="worker-loader-success"
        >
          <p className="title">Error!</p>
          <p className="sub-title">
            Yubikey provisioning for {`${user?.first_name} ${user?.last_name}`} has
            failed.
          </p>
          <MessageBox type={MessageBoxType.ERROR}>
            <p>{errorData}</p>
          </MessageBox>
          <div className="controls">
            <Button
              size={ButtonSize.LARGE}
              styleVariant={ButtonStyleVariant.STANDARD}
              text="Close"
              className="cancel"
              onClick={() => setIsOpen(false)}
            />
          </div>
        </motion.div>
      ) : null}
    </AnimatePresence>
  );
};

export default WorkerLoader;
