import { m } from '../../paraglide/messages';
import api from '../api/api';
import type { Device } from '../api/types';
import { Snackbar } from '../defguard-ui/providers/snackbar/snackbar';
import { openModal } from '../hooks/modalControls/modalsSubjects';
import { ModalName } from '../hooks/modalControls/modalTypes';

export type UserDeviceContext = {
  device: Device;
  reservedNames: string[];
  reservedPubkeys: string[];
};

export const userDeviceActions = {
  fetchContext: async (
    username: string,
    deviceId: number,
  ): Promise<UserDeviceContext | null> => {
    try {
      const { data } = await api.user.getUser(username);
      const device = data.devices.find(({ id }) => id === deviceId);
      if (!device) {
        Snackbar.error(m.devices_load_error());
        return null;
      }
      return {
        device,
        reservedNames: data.devices.map(({ name }) => name),
        reservedPubkeys: data.devices.map(({ wireguard_pubkey }) => wireguard_pubkey),
      };
    } catch (error) {
      Snackbar.error(m.devices_load_error());
      console.error(error);
      return null;
    }
  },
  openEdit: (
    username: string,
    { device, reservedNames, reservedPubkeys }: UserDeviceContext,
  ) => {
    openModal(ModalName.EditUserDevice, {
      device,
      reservedNames,
      reservedPubkeys,
      username,
      hidePubkey: false,
    });
  },
  openIpSettings: (username: string, device: Device) => {
    api.device
      .getDeviceIps(username, device.id)
      .then(({ data: locationData }) => {
        openModal(ModalName.AssignUserDeviceIP, {
          device,
          username,
          locationData,
        });
      })
      .catch((error) => {
        Snackbar.error(m.profile_devices_ip_settings_load_failed());
        console.error(error);
      });
  },
  openConfig: (device: Device) => {
    api.device.getDeviceConfigs(device).then((modalData) => {
      const hasConfigs = modalData.configs.some(
        (c) => !c.mfa_enabled && !c.posture_check_required,
      );
      if (!hasConfigs) {
        Snackbar.error(m.profile_devices_config_no_locations());
        return;
      }
      openModal(ModalName.UserDeviceConfig, modalData);
    });
  },
} as const;
