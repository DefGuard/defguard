// import './style.scss';

// import classNames from 'classnames';
// import dayjs from 'dayjs';
// import utc from 'dayjs/plugin/utc';
// import { TargetAndTransition } from 'framer-motion';
// import { isUndefined, orderBy } from 'lodash-es';
// import { useMemo, useState } from 'react';

import { useI18nContext } from '../../../../../i18n/i18n-react';
// import IconClip from '../../../../../shared/components/svg/IconClip';
// import SvgIconCollapse from '../../../../../shared/components/svg/IconCollapse';
// import SvgIconExpand from '../../../../../shared/components/svg/IconExpand';
// import { ColorsRGB } from '../../../../../shared/constants';
// import { Badge } from '../../../../../shared/defguard-ui/components/Layout/Badge/Badge';
import { Card } from '../../../../../shared/defguard-ui/components/Layout/Card/Card';
// import { DeviceAvatar } from '../../../../../shared/defguard-ui/components/Layout/DeviceAvatar/DeviceAvatar';
// import { EditButton } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
// import { EditButtonOption } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
// import { EditButtonOptionStyleVariant } from '../../../../../shared/defguard-ui/components/Layout/EditButton/types';
import { Label } from '../../../../../shared/defguard-ui/components/Layout/Label/Label';
// import { NoData } from '../../../../../shared/defguard-ui/components/Layout/NoData/NoData';
// import { useUserProfileStore } from '../../../../../shared/hooks/store/useUserProfileStore';
import { OpenIdProvider } from '../../../../../shared/types';
// import { sortByDate } from '../../../../../shared/utils/sortByDate';
// import { useDeleteDeviceModal } from '../hooks/useDeleteDeviceModal';
// import { useDeviceConfigModal } from '../hooks/useDeviceConfigModal';
// import { useEditDeviceModal } from '../hooks/useEditDeviceModal';

// dayjs.extend(utc);

// const dateFormat = 'DD.MM.YYYY | HH:mm';

// const formatDate = (date: string): string => {
//   return dayjs.utc(date).format(dateFormat);
// };

interface Props {
  provider: OpenIdProvider;
}

export const ProviderDetails = ({ provider }: Props) => {
  // const [hovered, setHovered] = useState(false);
  const { LL } = useI18nContext();
  // const setDeleteDeviceModal = useDeleteDeviceModal((state) => state.setState);
  // const setEditDeviceModal = useEditDeviceModal((state) => state.setState);
  // const openDeviceConfigModal = useDeviceConfigModal((state) => state.open);

  // const schema = useMemo(
  //   () =>
  //     z.object({
  //       name: z.string().min(1, LL.form.error.required()),
  //       document_url: z
  //         .string()
  //         .url(LL.form.error.invalid())
  //         .min(1, LL.form.error.required()),
  //     }),
  //   [LL.form.error],
  // );

  // const getContainerAnimate = useMemo((): TargetAndTransition => {
  //   const res: TargetAndTransition = {
  //     borderColor: ColorsRGB.White,
  //   };
  //   if (expanded || hovered) {
  //     res.borderColor = ColorsRGB.GrayBorder;
  //   }
  //   return res;
  // }, [expanded, hovered]);

  // // first, order by last_connected_at then if not preset, by network_id
  // const orderedLocations = useMemo((): DeviceNetworkInfo[] => {
  //   const connected = device.networks.filter(
  //     (network) => !isUndefined(network.last_connected_at),
  //   );

  //   const neverConnected = device.networks.filter((network) =>
  //     isUndefined(network.last_connected_at),
  //   );

  //   const connectedSorted = sortByDate(
  //     connected,
  //     (n) => n.last_connected_at as string,
  //     true,
  //   );
  //   const neverConnectedSorted = orderBy(neverConnected, ['network_id'], ['desc']);

  //   return [...connectedSorted, ...neverConnectedSorted];
  // }, [device.networks]);

  // const latestLocation = orderedLocations.length ? orderedLocations[0] : undefined;

  // if (!user) return null;

  return (
    <Card
      // className={cn}
      initial={false}
      // animate={getContainerAnimate}
      // onMouseOver={() => setHovered(true)}
      // onMouseOut={() => setHovered(false)}
    >
      <section className="main-info">
        <header>
          <h3 data-testid="provider-name">{provider.name}</h3>
        </header>
        <div className="section-content">
          <div>
            <Label>{LL.settingsPage.openIdSettings.form.labels.documentUrl()}</Label>
            <p data-testid="device-last-connected-from">{provider.document_url}</p>
          </div>
        </div>
      </section>
      <div className="card-controls">
        {/* <EditButton visible={true}> */}
        {/*   <EditButtonOption */}
        {/*     text={LL.userPage.devices.card.edit.edit()} */}
        {/*     onClick={() => { */}
        {/*       setEditDeviceModal({ */}
        {/*         visible: true, */}
        {/*         device: device, */}
        {/*       }); */}
        {/*     }} */}
        {/*   /> */}
        {/*   <EditButtonOption */}
        {/*     styleVariant={EditButtonOptionStyleVariant.STANDARD} */}
        {/*     text={LL.userPage.devices.card.edit.showConfigurations()} */}
        {/*     disabled={!device.networks?.length} */}
        {/*     onClick={() => { */}
        {/*       openDeviceConfigModal({ */}
        {/*         deviceName: device.name, */}
        {/*         publicKey: device.wireguard_pubkey, */}
        {/*         deviceId: device.id, */}
        {/*         userId: user.user.id, */}
        {/*         networks: device.networks.map((n) => ({ */}
        {/*           networkId: n.network_id, */}
        {/*           networkName: n.network_name, */}
        {/*         })), */}
        {/*       }); */}
        {/*     }} */}
        {/*   /> */}
        {/*   <EditButtonOption */}
        {/*     styleVariant={EditButtonOptionStyleVariant.WARNING} */}
        {/*     text={LL.userPage.devices.card.edit.delete()} */}
        {/*     onClick={() => */}
        {/*       setDeleteDeviceModal({ */}
        {/*         visible: true, */}
        {/*         device: device, */}
        {/*       }) */}
        {/*     } */}
        {/*   /> */}
        {/* </EditButton> */}
        {/* <ExpandButton */}
        {/*   expanded={expanded} */}
        {/*   onClick={() => setExpanded((state) => !state)} */}
        {/* /> */}
      </div>
    </Card>
  );
};
