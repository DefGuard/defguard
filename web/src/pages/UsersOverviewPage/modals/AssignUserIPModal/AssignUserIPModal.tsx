import { useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import axios from 'axios';
import { useCallback, useEffect, useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import type {
  LocationDevices,
  LocationDevicesResponse,
} from '../../../../shared/api/types';
import { IpAssignmentCard } from '../../../../shared/components/IpAssignmentCard/IpAssignmentCard';
import { IpAssignmentDeviceSection } from '../../../../shared/components/IpAssignmentDeviceSection/IpAssignmentDeviceSection';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SuggestedIpInput } from '../../../../shared/defguard-ui/components/SuggestedIPInput/SuggestedIPInput';
import { Snackbar } from '../../../../shared/defguard-ui/providers/snackbar/snackbar';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../../shared/form';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import type { OpenAssignUserIPModal } from '../../../../shared/hooks/modalControls/types';
import './style.scss';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';

const modalNameValue = ModalName.AssignUserIP;

type ModalData = OpenAssignUserIPModal;

const formSchema = z.object({
  locations: z.array(
    z.object({
      location_id: z.number(),
      devices: z.array(
        z.object({
          device_id: z.number(),
          ips: z.array(
            z.object({
              modifiable_part: z.string().trim(),
              network_part: z.string(),
            }),
          ),
        }),
      ),
    }),
  ),
});

type FormFields = z.infer<typeof formSchema>;

export const AssignUserIPModal = () => {
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
      id="assign-user-ip-modal"
      title={
        isPresent(modalData)
          ? m.modal_assign_user_ip_title({
              firstName: modalData.user.first_name,
              lastName: modalData.user.last_name,
            })
          : m.modal_assign_user_ip_title_fallback()
      }
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

const ModalContent = ({ user, locationData, hasDevices }: ModalData) => {
  return (
    <AssignmentForm
      username={user.username}
      firstName={user.first_name}
      lastName={user.last_name}
      locationData={locationData}
      hasDevices={hasDevices}
    />
  );
};

type AssignmentFormProps = {
  username: string;
  firstName: string;
  lastName: string;
  locationData: LocationDevicesResponse;
  hasDevices: boolean;
};

const AssignmentForm = ({
  username,
  firstName,
  lastName,
  locationData,
  hasDevices,
}: AssignmentFormProps) => {
  const [openLocations, setOpenLocations] = useState<Set<number>>(() => new Set());

  const defaultValues: FormFields = useMemo(
    () => ({
      locations: locationData.locations.map((loc) => ({
        location_id: loc.location_id,
        devices: loc.devices.map((dev) => ({
          device_id: dev.device_id,
          ips: dev.wireguard_ips.map((ip) => ({
            modifiable_part: ip.modifiable_part,
            network_part: ip.network_part,
          })),
        })),
      })),
    }),
    [locationData],
  );

  const { mutateAsync: updateDevices } = useMutation({
    mutationFn: (formData: FormFields) => {
      const assignments = formData.locations
        .flatMap((loc) =>
          loc.devices.map((dev) => ({
            device_id: dev.device_id,
            location_id: loc.location_id,
            ips: dev.ips
              .filter((e) => e.modifiable_part.length > 0)
              .map((e) => `${e.network_part}${e.modifiable_part}`),
          })),
        )
        .filter((a) => a.ips.length > 0);
      return api.device.assignUserDeviceIps(username, assignments);
    },
    meta: {
      invalidate: [['user-device-ips', username]],
    },
    onSuccess: () => {
      Snackbar.default(m.modal_assign_user_ip_success({ firstName, lastName }));
      closeModal(modalNameValue);
    },
    onError: (error) => {
      console.error('Failed to update IP addresses:', error);
      Snackbar.error(m.modal_assign_user_ip_error());
    },
  });

  const form = useAppForm({
    defaultValues,
    validators: {
      onSubmit: formSchema,
    },
    onSubmit: async ({ value }) => {
      await updateDevices(value);
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
    async (value: string, deviceId: number, locationId: number) => {
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
    [username],
  );

  return (
    <form.AppForm>
      <div className="assign-user-ip-modal">
        <div className="assignment-mode">
          <h3>{m.modal_assign_user_ip_assignment_mode_title()}</h3>
          <SizedBox height={ThemeSpacing.Xl} />
          <p className="mode-description">
            {m.modal_assign_user_ip_assignment_mode_description()}
          </p>
        </div>

        <div className="devices-list">
          {locationData.locations.length === 0 && (
            <p className="no-locations">
              {hasDevices
                ? m.modal_assign_user_ip_no_locations()
                : m.modal_assign_user_ip_no_devices()}
            </p>
          )}
          {locationData.locations.map((location: LocationDevices, locIdx) => (
            <IpAssignmentCard
              key={location.location_id}
              title={location.location_name}
              isOpen={openLocations.has(location.location_id)}
              onOpenChange={() => toggleLocation(location.location_id)}
            >
              {location.devices.map((deviceIps, devIdx) => (
                <IpAssignmentDeviceSection
                  key={deviceIps.device_id}
                  name={deviceIps.device_name}
                >
                  {deviceIps.wireguard_ips.map((ipData, ipIdx) => (
                    <form.Field
                      key={`${deviceIps.device_id}-${ipData.ip}`}
                      name={`locations[${locIdx}].devices[${devIdx}].ips[${ipIdx}].modifiable_part`}
                      validators={{
                        onChangeAsyncDebounceMs: 200,
                        onChangeAsync: ({ value }) =>
                          validateIp(
                            `${ipData.network_part}${value}`,
                            deviceIps.device_id,
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
              ))}
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
