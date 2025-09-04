import { useEffect, useMemo } from 'react';
import { shallow } from 'zustand/shallow';
import { Button } from '../../../defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../defguard-ui/components/Layout/Button/types';
import { Modal } from '../../../defguard-ui/components/Layout/modals/Modal/Modal';
import { OutdatedComponent, type OutdatedComponentInfo } from '../../../types';
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

type ItemProps = {
  data: OutdatedComponentInfo;
};

const ListItem = ({ data }: ItemProps) => {
  return (
    <li>
      <div>
        {`${data.component}`}
        <span>-</span>
        <span className="version">{data.version}</span>
      </div>
    </li>
  );
};

const ModalContent = () => {
  const closeModal = useOutdatedComponentsModal((s) => s.close);
  const componentsInfo = useOutdatedComponentsModal((s) =>
    s.componentsInfo.filter((c) => !c.is_supported),
  );
  const { LL } = useI18nContext();
  const localLL = LL.modals.outdatedComponentsModal;

  const gatewaysInfo = useMemo(
    () =>
      componentsInfo.filter(
        (component) => component.component === OutdatedComponent.GATEWAY,
      ),
    [componentsInfo],
  );

  const proxyInfo = useMemo(
    () => componentsInfo.find((c) => c.component === OutdatedComponent.PROXY),
    [componentsInfo],
  );

  useEffect(() => {
    if (componentsInfo.length === 0) {
      closeModal();
    }
  }, [closeModal, componentsInfo.length]);

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
            {isPresent(proxyInfo) && <ListItem data={proxyInfo} />}
            {gatewaysInfo.map((gateway, index) => (
              <ListItem key={index} data={gateway} />
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
