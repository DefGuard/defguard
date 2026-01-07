import { useQuery } from '@tanstack/react-query';
import { Modal } from '../../../defguard-ui/components/Modal/Modal';
import { isPresent } from '../../../defguard-ui/utils/isPresent';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../hooks/modalControls/modalTypes';
import type { OpenGatewaySetupModal } from '../../../hooks/modalControls/types';
import './style.scss';
import { useEffect, useMemo, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../api/api';
import { Badge } from '../../../defguard-ui/components/Badge/Badge';
import { Button } from '../../../defguard-ui/components/Button/Button';
import { CopyField } from '../../../defguard-ui/components/CopyField/CopyField';
import { Divider } from '../../../defguard-ui/components/Divider/Divider';
import { MarkedSection } from '../../../defguard-ui/components/MarkedSection/MarkedSection';
import { SizedBox } from '../../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../defguard-ui/types';
import { DescriptionBlock } from '../../DescriptionBlock/DescriptionBlock';

const modalNameValue = ModalName.GatewaySetup;

type ModalData = OpenGatewaySetupModal & {
  initialGw: string[];
};

export const GatewaySetupModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameValue, async (data) => {
      const { data: gwStatus } = await api.location.getLocationGatewaysStatus(
        data.networkId,
      );

      setModalData({
        ...data,
        initialGw: gwStatus.map((gw) => gw.uid),
      });
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameValue, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal
      id="gateway-setup-modal"
      title={'Gateway Setup'}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {
        setModalData(null);
      }}
    >
      {isPresent(modalData) && <ModalContent {...modalData} />}
    </Modal>
  );
};

const ModalContent = ({ data, networkId, initialGw }: ModalData) => {
  const {
    data: gwStatus,
    refetch: refetchGwStatus,
    isRefetching,
  } = useQuery({
    queryFn: () => api.location.getLocationGatewaysStatus(networkId),
    queryKey: ['network', networkId, 'gateways'],
    select: (resp) => resp.data,
    refetchInterval: 60_000,
  });

  const isNewConnected = useMemo(() => {
    if (gwStatus) {
      const newGw = gwStatus.find((gw) => !initialGw.includes(gw.uid));
      return isPresent(newGw);
    }
    return false;
  }, [gwStatus, initialGw]);

  return (
    <>
      <MarkedSection icon="code">
        <DescriptionBlock title="Authentication token">
          <p>Use the token below to authenticate and configure your gateway node.</p>
        </DescriptionBlock>
        <SizedBox height={ThemeSpacing.Xl2} />
        <CopyField
          text={data.grpc_url}
          label="URL"
          copyTooltip={m.misc_clipboard_copy()}
        />
        <SizedBox height={ThemeSpacing.Xl} />
        <CopyField
          text={data.token}
          label="Authentication Token"
          copyTooltip={m.misc_clipboard_copy()}
        />
        <Divider spacing={ThemeSpacing.Xl2} />
      </MarkedSection>
      <MarkedSection icon="online">
        <DescriptionBlock title="Connection status">
          <p>
            Once everything is set up and your token is entered, check the connection
            status below. If it still fails, review the gateway logs.
          </p>
        </DescriptionBlock>
        <SizedBox height={ThemeSpacing.Xl2} />
        <div className="connection-status">
          {isNewConnected && (
            <Badge variant="success" icon="status-simple" text="Connected" showIcon />
          )}
          {!isNewConnected && (
            <Badge
              variant="warning"
              icon="status-attention"
              text="Not Connected"
              showIcon
            />
          )}
          <Button
            text="Check connection"
            iconLeft="refresh"
            variant="outlined"
            loading={isRefetching}
            onClick={() => {
              refetchGwStatus();
            }}
          />
        </div>
      </MarkedSection>
    </>
  );
};
