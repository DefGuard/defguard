import { useEffect, useMemo } from 'react';
import { shallow } from 'zustand/shallow';
import { Button } from '../../../defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../defguard-ui/components/Layout/Button/types';
import { Modal } from '../../../defguard-ui/components/Layout/modals/Modal/Modal';
import type { OutdatedGateway, OutdatedProxy } from '../../../types';
import { useOutdatedComponentsModal } from './useOutdatedComponentsModal';
import './style.scss';
import { useI18nContext } from '../../../../i18n/i18n-react';
import { isPresent } from '../../../defguard-ui/utils/isPresent';

export const OutdatedComponentsModal = () => {
  const isOpen = useOutdatedComponentsModal((s) => s.visible);
  const [close, reset] = useOutdatedComponentsModal((s) => [s.close, s.reset], shallow);
  return (
    <Modal
      isOpen={isOpen}
      id="outdated-components-modal"
      className="outdated-components-modal"
      onClose={close}
      afterClose={reset}
      disableClose
    >
      <ModalContent />
    </Modal>
  );
};

type ProxyItemProps = {
  data: OutdatedProxy;
};

const ProxyListItem = ({ data }: ProxyItemProps) => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.outdatedComponentsModal.content;
  return (
    <li>
      <div>
        Proxy
        <span>-</span>
        <span className="version">{data.version || localLL.unknownVersion()}</span>
      </div>
    </li>
  );
};

type GatewayItemProps = {
  data: OutdatedGateway;
};

const GatewayListItem = ({ data }: GatewayItemProps) => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.outdatedComponentsModal.content;
  return (
    <li>
      <div>
        Gateway
        <span>-</span>
        <span className="version">
          {data.version || localLL.unknownVersion()} (
          {data.hostname || localLL.unknownHostname()})
        </span>
      </div>
    </li>
  );
};

const ModalContent = () => {
  const closeModal = useOutdatedComponentsModal((s) => s.close);
  const { LL } = useI18nContext();
  const localLL = LL.modals.outdatedComponentsModal;
  const componentsInfo = useOutdatedComponentsModal((s) => s);

  const gatewaysInfo = useMemo(
    () => componentsInfo.componentsInfo.gateways,
    [componentsInfo],
  );

  const proxyInfo = useMemo(() => componentsInfo.componentsInfo.proxy, [componentsInfo]);

  useEffect(() => {
    if (gatewaysInfo.length === 0 && proxyInfo === undefined) {
      closeModal();
    }
  }, [closeModal, gatewaysInfo.length, proxyInfo]);

  return (
    <div className="content-wrapper">
      <div className="top">
        <h1 className="title">{localLL.title()}</h1>
        <h3 className="subtitle">{localLL.subtitle()}</h3>
      </div>
      <div className="bottom">
        <div className="content">
          <h2>{localLL.content.title()}</h2>
          <ul>
            {isPresent(proxyInfo) && <ProxyListItem data={proxyInfo} />}
            {gatewaysInfo.map((gateway, index) => (
              <GatewayListItem key={index} data={gateway} />
            ))}
          </ul>
        </div>
        <div className="controls">
          <Button
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.STANDARD}
            text={LL.common.controls.dismiss()}
            onClick={() => {
              closeModal();
            }}
          />
        </div>
      </div>
    </div>
  );
};
