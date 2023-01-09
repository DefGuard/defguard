import './style.scss';

import { useMemo, useState } from 'react';
import useBreakpoint from 'use-breakpoint';

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
import { deviceBreakpoints } from '../../../../../shared/constants';
import { displayDate } from '../../../../../shared/helpers/displayDate';
import { useModalStore } from '../../../../../shared/hooks/store/useModalStore';
import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { Device } from '../../../../../shared/types';
import { downloadWGConfig } from '../../../../../shared/utils/downloadWGConfig';

interface Props {
  device: Device;
}

export const DeviceCard = ({ device }: Props) => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const user = useUserProfileStore((state) => state.user);
  const { breakpoint } = useBreakpoint(deviceBreakpoints);
  const [editButtonVisible, setEditButtonVisible] = useState(false);
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

  const formattedCreationDate = useMemo(
    () => displayDate(device.created),
    [device]
  );

  if (!user) return null;

  return (
    <Card
      className="device-card"
      onHoverStart={() => {
        setEditButtonVisible(true);
      }}
      onHoverEnd={() => {
        setEditButtonVisible(false);
      }}
    >
      <header>
        <AvatarBox>
          <DeviceAvatar deviceId={Number(device.id)} />
        </AvatarBox>
        <h3 data-test="device-name">{device.name}</h3>
      </header>
      <div className="content">
        <div className="info">
          <Label>{LL.userPage.devices.card.labels.location()}</Label>
          <p data-text="device-location">Szczecin</p>
        </div>
        <div className="info">
          <Label>{LL.userPage.devices.card.labels.lastIpAddress()}</Label>
          <p>{device.wireguard_ip}</p>
        </div>
        <div className="info">
          <Label>{LL.userPage.devices.card.labels.date()}</Label>
          <p>{formattedCreationDate}</p>
        </div>
      </div>
      <EditButton visible={editButtonVisible || breakpoint !== 'desktop'}>
        <EditButtonOption
          text={LL.userPage.devices.card.edit.edit()}
          onClick={() => {
            setModalsState({
              editUserDeviceModal: { visible: true, device: device },
            });
          }}
        />
        <EditButtonOption
          text={LL.userPage.devices.card.edit.download()}
          onClick={() => handleDownload()}
        />
        <EditButtonOption
          styleVariant={EditButtonOptionStyleVariant.WARNING}
          text={LL.userPage.devices.card.edit.delete()}
          onClick={() =>
            setDeleteUserDeviceModal({ visible: true, device: device })
          }
        />
      </EditButton>
    </Card>
  );
};
