import './style.scss';

import { useMemo, useState } from 'react';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { AvatarBox } from '../../../../../shared/components/layout/AvatarBox/AvatarBox';
import { Card } from '../../../../../shared/components/layout/Card/Card';
import { DeviceAvatar } from '../../../../../shared/components/layout/DeviceAvatar/DeviceAvatar';
import { EditButton } from '../../../../../shared/components/layout/EditButton/EditButton';
import {
  EditButtonOption,
  EditButtonOptionStyleVariant,
} from '../../../../../shared/components/layout/EditButton/EditButtonOption';
import { Label } from '../../../../../shared/components/layout/Label/Label';
import { Tag } from '../../../../../shared/components/layout/Tag/Tag';
import { IconClip, IconCollapse, IconExpand } from '../../../../../shared/components/svg';
import { displayDate } from '../../../../../shared/helpers/displayDate';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { Device } from '../../../../../shared/types';
import { downloadWGConfig } from '../../../../../shared/utils/downloadWGConfig';
import { range } from 'lodash-es';
import Badge from '../../../../../shared/components/layout/Badge/Badge';

interface Props {
  device: Device;
}

export const DeviceCard = ({ device }: Props) => {
  const [expanded, setExpanded] = useState(false);
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const user = useUserProfileStore((state) => state.user);
  const setDeleteUserDeviceModal = useModalStore(
    (state) => state.setDeleteUserDeviceModal
  );
  const setModalsState = useModalStore((state) => state.setState);
  const {
    device: { downloadDeviceConfig },
  } = useApi();

  const handleDownload = () => {
    downloadDeviceConfig(device.id)
      .then((res) => {
        downloadWGConfig(res, device.name);
      })
      .catch((err) => {
        toaster.error(LL.messages.clipboardError());
        console.error(err);
      });
  };

  const formattedCreationDate = useMemo(() => displayDate(device.created), [device]);

  if (!user) return null;

  return (
    <Card className="device-card">
      <div className="content-container">
        <section className="main-info">
          <header>
            <AvatarBox>
              <DeviceAvatar deviceId={Number(device.id)} />
            </AvatarBox>
            <h3 data-testid="device-name">{device.name}</h3>
          </header>
          <div className="section-content">
            <div>
              <Label>{LL.userPage.devices.card.labels.lastConnected()}</Label>
              <p>{formattedCreationDate}</p>
            </div>
            <div>
              <Label>{LL.userPage.devices.card.labels.assignedIp()}</Label>
              <p>{device.wireguard_ip}</p>
            </div>
          </div>
        </section>
        {range(0, 2, 1).map((index) => (
          <DeviceLocation key={index} />
        ))}
      </div>
      <div className="card-controls">
        <EditButton visible={true}>
          <EditButtonOption
            text={LL.userPage.devices.card.edit.edit()}
            onClick={() => {
              setModalsState({
                editUserDeviceModal: { visible: true, device: device },
              });
            }}
          />
          <EditButtonOption
            text={LL.userPage.devices.card.edit.downloadConfig({
              name: 'PLACEHOLDER',
            })}
            onClick={() => handleDownload()}
          />
          <EditButtonOption
            styleVariant={EditButtonOptionStyleVariant.WARNING}
            text={LL.userPage.devices.card.edit.delete()}
            onClick={() => setDeleteUserDeviceModal({ visible: true, device: device })}
          />
        </EditButton>
        <ExpandButton
          expanded={expanded}
          onClick={() => setExpanded((state) => !state)}
        />
      </div>
    </Card>
  );
};

const DeviceLocation = () => {
  const { LL } = useI18nContext();
  return (
    <div className="location">
      <header>
        <IconClip />
        <h2>Zurich</h2>
        <Badge text={'10.10.4.4'} />
      </header>
      <div className="section-content">
        <div>
          <Label>{LL.userPage.devices.card.labels.lastConnected()}</Label>
          <p>13.06.20223 | 09:12</p>
        </div>
        <div>
          <Label>{LL.userPage.devices.card.labels.assignedIp()}</Label>
          <p>10.6.0.1</p>
        </div>
      </div>
    </div>
  );
};

type ExpandButtonProps = {
  expanded: boolean;
  onClick: () => void;
};

const ExpandButton = ({ expanded, onClick }: ExpandButtonProps) => {
  return (
    <button className="device-card-expand" onClick={onClick}>
      {expanded ? <IconExpand /> : <IconCollapse />}
    </button>
  );
};
