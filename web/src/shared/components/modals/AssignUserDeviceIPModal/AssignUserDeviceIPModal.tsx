import { useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import axios from 'axios';
import { useCallback, useEffect, useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../../../paraglide/messages';
import api from '../../../api/api';
import type { DeviceLocationIp, DeviceLocationIpsResponse } from '../../../api/types';
import { Modal } from '../../../defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../defguard-ui/components/ModalControls/ModalControls';
import { SuggestedIpInput } from '../../../defguard-ui/components/SuggestedIPInput/SuggestedIPInput';
import { Snackbar } from '../../../defguard-ui/providers/snackbar/snackbar';
import { isPresent } from '../../../defguard-ui/utils/isPresent';
import { useAppForm } from '../../../form';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../hooks/modalControls/modalTypes';
import type { OpenAssignUserDeviceIPModal } from '../../../hooks/modalControls/types';
import { IpAssignmentCard } from '../../IpAssignmentCard/IpAssignmentCard';
import { IpAssignmentDeviceSection } from '../../IpAssignmentDeviceSection/IpAssignmentDeviceSection';
import './style.scss';

const modalNameValue = ModalName.AssignUserDeviceIP;

type ModalData = OpenAssignUserDeviceIPModal;

const formSchema = z.object({
  locations: z.array(
    z.object({
      location_id: z.number(),
      ips: z.array(
        z.object({
          modifiable_part: z.string().trim(),
          network_part: z.string(),
        }),
      ),
    }),
  ),
});

type FormFields = z.infer<typeof formSchema>;

export const AssignUserDeviceIPModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<ModalData | null>(null);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameValue, (data) => {
      setModalData(data);
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
      id="assign-user-device-ip-modal"
      title={m.modal_assign_user_device_ip_title()}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {
        setModalData(null);
      }}
    >
      {isPresent(modalData) && (
        <AssignmentForm
          deviceId={modalData.device.id}
          deviceName={modalData.device.name}
          username={modalData.username}
          locationData={modalData.locationData}
        />
      )}
    </Modal>
  );
};

type AssignmentFormProps = {
  deviceId: number;
  deviceName: string;
  username: string;
  locationData: DeviceLocationIpsResponse;
};

const AssignmentForm = ({
  deviceId,
  deviceName,
  username,
  locationData,
}: AssignmentFormProps) => {
  const [openLocations, setOpenLocations] = useState<Set<number>>(() => new Set());

  const defaultValues: FormFields = useMemo(
    () => ({
      locations: locationData.locations.map((loc) => ({
        location_id: loc.location_id,
        ips: loc.wireguard_ips.map((ip) => ({
          modifiable_part: ip.modifiable_part,
          network_part: ip.network_part,
        })),
      })),
    }),
    [locationData],
  );

  const { mutateAsync: updateDevice } = useMutation({
    mutationFn: (formData: FormFields) => {
      const assignments = formData.locations
        .map((loc) => ({
          device_id: deviceId,
          location_id: loc.location_id,
          ips: loc.ips
            .filter((ip) => ip.modifiable_part.length > 0)
            .map((ip) => `${ip.network_part}${ip.modifiable_part}`),
        }))
        .filter((a) => a.ips.length > 0);
      return api.device.assignUserDeviceIps(username, assignments);
    },
    meta: {
      invalidate: [
        ['user-device-ips', username],
        ['user', username],
      ],
    },
    onSuccess: () => {
      Snackbar.default(m.modal_assign_user_device_ip_success({ deviceName }));
      closeModal(modalNameValue);
    },
    onError: (error) => {
      console.error('Failed to update IP addresses:', error);
      Snackbar.error(m.modal_assign_user_device_ip_error());
    },
  });

  const form = useAppForm({
    defaultValues,
    validators: {
      onSubmit: formSchema,
    },
    onSubmit: async ({ value }) => {
      await updateDevice(value);
    },
  });

  const isSubmitting = useStore(form.store, (s) => s.isSubmitting);

  const toggleLocation = (locationId: number) => {
    setOpenLocations((prev) => {
      const next = new Set(prev);
      if (next.has(locationId)) {
        next.delete(locationId);
      } else {
        next.add(locationId);
      }
      return next;
    });
  };

  const validateIp = useCallback(
    async (value: string, locationId: number) => {
      try {
        await api.device.validateUserDeviceIp(username, {
          device_id: deviceId,
          ip: value,
          location: locationId,
        });
        return undefined;
      } catch (e) {
        return axios.isAxiosError(e)
          ? (e.response?.data?.msg ?? m.modal_assign_user_ip_validation_error())
          : m.modal_assign_user_ip_validation_error();
      }
    },
    [username, deviceId],
  );

  return (
    <form.AppForm>
      <div className="assign-user-device-ip-modal">
        <div className="device-ip-card">
          <h3 className="card-title">
            {m.modal_assign_user_device_ip_card_title({ deviceName })}
          </h3>
          <p className="card-description">
            {m.modal_assign_user_device_ip_assignment_description()}
          </p>
        </div>

        <div className="locations-list">
          {locationData.locations.length === 0 && (
            <p className="no-locations">{m.modal_assign_user_ip_no_locations()}</p>
          )}
          {locationData.locations.map((location: DeviceLocationIp, locIdx) => (
            <IpAssignmentCard
              key={location.location_id}
              title={location.location_name}
              isOpen={openLocations.has(location.location_id)}
              onOpenChange={() => toggleLocation(location.location_id)}
            >
              <IpAssignmentDeviceSection>
                {location.wireguard_ips.map((ipData, ipIdx) => (
                  <form.Field
                    key={`${deviceId}-${ipData.ip}`}
                    name={`locations[${locIdx}].ips[${ipIdx}].modifiable_part`}
                    validators={{
                      onChangeAsyncDebounceMs: 200,
                      onChangeAsync: ({ value }) =>
                        validateIp(
                          `${ipData.network_part}${value}`,
                          location.location_id,
                        ),
                    }}
                  >
                    {(field) => (
                      <SuggestedIpInput
                        data={ipData}
                        value={field.state.value}
                        loading={field.state.meta.isValidating}
                        error={field.state.meta.errors[0]?.toString()}
                        onChange={(val) => field.handleChange(val ?? '')}
                        onBlur={field.handleBlur}
                      />
                    )}
                  </form.Field>
                ))}
              </IpAssignmentDeviceSection>
            </IpAssignmentCard>
          ))}
        </div>

        <ModalControls
          submitProps={{
            text: m.controls_submit(),
            disabled: isSubmitting,
            onClick: () => form.handleSubmit(),
          }}
          cancelProps={{
            text: m.controls_cancel(),
            disabled: isSubmitting,
            onClick: () => closeModal(modalNameValue),
          }}
        />
      </div>
    </form.AppForm>
  );
};
