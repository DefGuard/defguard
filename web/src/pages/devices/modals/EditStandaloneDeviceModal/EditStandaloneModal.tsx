import './style.scss';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo, useState } from 'react';
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
import { useAuthStore } from '../../../../shared/hooks/store/useAuthStore';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../shared/queries';
import { selectifyNetworks } from '../../../../shared/utils/form/selectifyNetwork';
import { invalidateMultipleQueries } from '../../../../shared/utils/invalidateMultipleQueries';
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
    standaloneDevice: { editDevice },
  } = useApi();

  const { mutateAsync } = useMutation({
    mutationFn: editDevice,
    onSuccess: () => {
      toaster.success(localLL.toasts.success());
      invalidateMultipleQueries(queryClient, [
        [QueryKeys.FETCH_USER_PROFILE, currentUserId],
        [QueryKeys.FETCH_STANDALONE_DEVICE_LIST],
      ]);
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

  const locationOptions = useMemo(() => {
    if (networks) {
      return selectifyNetworks(networks);
    }
    return [];
  }, [networks]);

  const defaultValues = useMemo(() => {
    if (locationOptions && device) {
      let modifiablePart = device.assigned_ips.split(device.split_ip.network_part)[1];

      if (modifiablePart === undefined) {
        modifiablePart = device.split_ip.modifiable_part;
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
  }, [device, locationOptions]);

  const handleSubmit = useCallback(
    async (values: AddStandaloneDeviceFormFields) => {
      if (device) {
        await mutateAsync({
          assigned_ips: values.modifiableIpPart,
          id: device.id,
          name: values.name,
          description: values.description,
        });
      }
    },
    [device, mutateAsync],
  );

  if (!device) {
    return null;
  }

  return (
    <>
      {defaultValues && (
        <StandaloneDeviceModalForm
          mode={StandaloneDeviceModalFormMode.EDIT}
          defaults={defaultValues}
          locationOptions={locationOptions}
          onLoadingChange={setFormLoading}
          onSubmit={handleSubmit}
          submitSubject={submitSubject}
          reservedNames={reservedDeviceNames}
          initialIpRecommendation={{
            ip: device.assigned_ips,
            ...device.split_ip,
          }}
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
