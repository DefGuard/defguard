import './style.scss';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useId, useMemo, useState } from 'react';
import { Subject } from 'rxjs';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { LoaderSpinner } from '../../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { ModalWithTitle } from '../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../shared/queries';
import { Network } from '../../../../shared/types';
import { selectifyNetworks } from '../../../../shared/utils/form/selectifyNetwork';
import { useDevicesPage } from '../../hooks/useDevicesPage';
import { useEditStandaloneDeviceModal } from '../../hooks/useEditStandaloneDeviceModal';
import {
  AddStandaloneDeviceFormFields,
  WGConfigGenChoice,
} from '../AddStandaloneDeviceModal/types';
import { StandaloneDeviceModalForm } from '../components/StandaloneDeviceModalForm/StandaloneDeviceModalForm';
import { StandaloneDeviceModalFormMode } from '../components/types';

export const EditStandaloneModal = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.editStandaloneModal;
  const [close, reset] = useEditStandaloneDeviceModal((s) => [s.close, s.reset], shallow);
  const isOpen = useEditStandaloneDeviceModal((s) => s.visible);

  useEffect(() => {
    return () => {
      reset();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <ModalWithTitle
      title={localLL.title()}
      isOpen={isOpen}
      onClose={close}
      afterClose={reset}
      id="edit-standalone-device-modal"
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const ModalContent = () => {
  // this is needs bcs opening modal again and again would prevent availableIp to refetch
  const modalSessionID = useId();
  const { LL } = useI18nContext();
  const localLL = LL.modals.editStandaloneModal;
  const device = useEditStandaloneDeviceModal((s) => s.device);
  const [submitSubject] = useState(new Subject<void>());
  const [formLoading, setFormLoading] = useState(false);
  const [closeModal] = useEditStandaloneDeviceModal((s) => [s.close], shallow);
  const toaster = useToaster();
  const queryClient = useQueryClient();
  const currentUserId = useAuthStore((s) => s.user?.id);
  const [{ reservedDeviceNames }] = useDevicesPage();

  const {
    network: { getNetworks },
    standaloneDevice: { getAvailableIp, editDevice },
  } = useApi();

  const { mutateAsync } = useMutation({
    mutationFn: editDevice,
    onSuccess: () => {
      toaster.success(localLL.toasts.success());
      queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_STANDALONE_DEVICE_LIST],
      });
      queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_USER_PROFILE, currentUserId],
      });
      closeModal();
    },
    onError: (e) => {
      toaster.error(localLL.toasts.failure());
      console.error(e);
    },
  });

  const { data: networks } = useQuery({
    queryKey: [QueryKeys.FETCH_NETWORKS],
    queryFn: getNetworks,
    refetchOnWindowFocus: false,
    refetchOnMount: true,
  });

  const { data: availableIpResponse } = useQuery({
    queryKey: [
      'ADD_STANDALONE_DEVICE_MODAL_FETCH_INITIAL_AVAILABLE_IP',
      networks,
      modalSessionID,
    ],
    queryFn: () =>
      getAvailableIp({
        locationId: (networks as Network[])[0].id,
      }),
    enabled: networks !== undefined && Array.isArray(networks) && networks.length > 0,
    refetchOnMount: true,
    refetchOnReconnect: true,
    refetchOnWindowFocus: false,
  });

  const locationOptions = useMemo(() => {
    if (networks) {
      return selectifyNetworks(networks);
    }
    return [];
  }, [networks]);

  const defaultValues = useMemo(() => {
    if (locationOptions && availableIpResponse && device) {
      let modifiablePart = device.assigned_ip.split(availableIpResponse.network_part)[1];

      if (modifiablePart === undefined) {
        modifiablePart = availableIpResponse.modifiable_part;
      }

      const res: AddStandaloneDeviceFormFields = {
        name: device?.name,
        modifiableIpPart: modifiablePart,
        location_id: device.location.id,
        description: device.description,
        generationChoice: WGConfigGenChoice.AUTO,
        wireguard_pubkey: '',
      };
      return res;
    }
    return undefined;
  }, [availableIpResponse, device, locationOptions]);

  const handleSubmit = useCallback(
    async (values: AddStandaloneDeviceFormFields) => {
      if (device) {
        await mutateAsync({
          assigned_ip: values.modifiableIpPart,
          id: device.id,
          name: values.name,
          description: values.description,
        });
      }
    },
    [device, mutateAsync],
  );

  return (
    <>
      {defaultValues && isPresent(availableIpResponse) && (
        <StandaloneDeviceModalForm
          mode={StandaloneDeviceModalFormMode.EDIT}
          defaults={defaultValues}
          locationOptions={locationOptions}
          onLoadingChange={setFormLoading}
          onSubmit={handleSubmit}
          submitSubject={submitSubject}
          reservedNames={reservedDeviceNames}
          initialIpRecommendation={availableIpResponse}
        />
      )}
      {!defaultValues && (
        <div className="loader">
          <LoaderSpinner size={124} />
        </div>
      )}
      <div className="controls">
        <Button
          className="cancel"
          onClick={() => closeModal()}
          text={LL.common.controls.cancel()}
        />
        <Button
          loading={formLoading}
          disabled={defaultValues === undefined}
          className="submit"
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          onClick={() => submitSubject.next()}
          text={LL.common.controls.submit()}
        />
      </div>
    </>
  );
};
