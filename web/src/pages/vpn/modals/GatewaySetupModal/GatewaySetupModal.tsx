import './style.scss';

import { useQuery } from '@tanstack/react-query';
import { motion } from 'framer-motion';
import React from 'react';
import shallow from 'zustand/shallow';

import Button, {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/components/layout/Button/Button';
import Modal from '../../../../shared/components/layout/Modal/Modal';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../shared/hooks/useApi';
import { QueryKeys } from '../../../../shared/queries';
import KeyBox from '../../../users/shared/components/KeyBox/KeyBox';

const GatewaySetupModal: React.FC = () => {
  const [modalState, setModalState] = useModalStore(
    (state) => [state.gatewaySetupModal, state.setGatewaySetupModal],
    shallow
  );
  const setIsOpen = (v: boolean) => {
    setModalState({ visible: v });
  };
  const {
    network: { getNetworkToken },
  } = useApi();
  const { data } = useQuery([QueryKeys.FETCH_NETWORK_TOKEN], () =>
    getNetworkToken('1')
  );

  if (!modalState.visible) return null;

  return (
    <Modal
      backdrop
      setIsOpen={(v: boolean) => setModalState({ visible: v })}
      className="gateway-setup middle"
      isOpen={modalState.visible}
    >
      <div className="header">
        <p>
          Gateway not connected. Please run command below to start your gateway
          server.
        </p>
      </div>
      <motion.ul layout className="commands-list">
        <motion.li layout>
          <KeyBox
            collapsible
            keyValue={`docker run -e DEFGUARD_TOKEN=${data?.token} registry.teonite.net/defguard/wireguard:latest`}
            title="Docker run command for gateway server"
          />
        </motion.li>
      </motion.ul>
      <div className="controls">
        <Button
          size={ButtonSize.BIG}
          styleVariant={ButtonStyleVariant.STANDARD}
          className="cancel"
          text="Close"
          onClick={() => setIsOpen(false)}
        />
      </div>
    </Modal>
  );
};

export default GatewaySetupModal;
