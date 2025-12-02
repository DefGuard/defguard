import { useNavigate } from '@tanstack/react-router';
import { useEffect, useState } from 'react';
import { LocationServiceMode } from '../../../../shared/api/types';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { SectionSelect } from '../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import { useAddLocationStore } from '../../../AddLocationPage/useAddLocationStore';

const modalNameValue = ModalName.AddLocation;

export const AddLocationModal = () => {
  const [isOpen, setOpen] = useState(false);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameValue, () => {
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameValue, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal title="Selection location type" isOpen={isOpen} onClose={() => setOpen(false)}>
      <ModalContent />
    </Modal>
  );
};

const ModalContent = () => {
  const navigate = useNavigate();
  return (
    <>
      <SectionSelect
        image="location"
        content="Set up your location manually by defining all configuration parameters yourself."
        title="Regular location"
        data-testid='add-regular-location'
        onClick={() => {
          useAddLocationStore.getState().start();
          navigate({
            to: '/add-location',
          });
        }}
      />
      <SizedBox height={ThemeSpacing.Md} />
      <SectionSelect
        image="service-location"
        content="Service locations are a special kind of locations that allow establishing automatic VPN connections on system boot."
        title="Service location (Windows only)"
        data-testid='add-service-location'
        onClick={() => {
          useAddLocationStore.getState().start({
            locationType: 'service',
            service_location_mode: LocationServiceMode.Prelogon,
          });
          navigate({
            to: '/add-location',
          });
        }}
      />
    </>
  );
};
