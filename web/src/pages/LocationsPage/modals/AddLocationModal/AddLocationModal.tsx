import { useNavigate } from '@tanstack/react-router';
import { useEffect, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import { LocationServiceMode } from '../../../../shared/api/types';
import { enterpriseBadgeProps } from '../../../../shared/components/badges/EnterpriseBadge';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { SectionSelect } from '../../../../shared/defguard-ui/components/SectionSelect/SectionSelect';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import type { OpenAddLocationModal } from '../../../../shared/hooks/modalControls/types';
import { canUseServiceLocations } from '../../../../shared/utils/license';
import { useAddLocationStore } from '../../../AddLocationPage/useAddLocationStore';

const modalNameValue = ModalName.AddLocation;

export const AddLocationModal = () => {
  const [modalData, setModalData] = useState<OpenAddLocationModal | null>(null);
  const [isOpen, setOpen] = useState(false);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameValue, (data) => {
      setOpen(true);
      setModalData(data);
    });
    const closeSub = subscribeCloseModal(modalNameValue, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <Modal
      title={m.modal_add_location_title()}
      isOpen={isOpen}
      onClose={() => {
        setOpen(false);
      }}
      afterClose={() => {
        setModalData(null);
      }}
    >
      {isPresent(modalData) && <AddLocationModalContent modalData={modalData} />}
    </Modal>
  );
};

export const AddLocationModalContent = ({
  modalData,
}: {
  modalData: OpenAddLocationModal;
}) => {
  const navigate = useNavigate();
  const serviceLocationsEnabled = canUseServiceLocations(modalData.license ?? null);

  return (
    <>
      <SectionSelect
        image="location"
        content={m.modal_add_location_regular_content()}
        title={m.modal_add_location_regular_title()}
        data-testid="add-regular-location"
        onClick={() => {
          useAddLocationStore.getState().start();
          navigate({
            to: '/add-location',
          });
        }}
      />
      <SizedBox height={ThemeSpacing.Md} />
      <SectionSelect
        badgeProps={!serviceLocationsEnabled ? enterpriseBadgeProps : undefined}
        image="service-location"
        content={m.modal_add_location_service_content()}
        title={m.modal_add_location_service_title()}
        data-testid="add-service-location"
        disabled={!serviceLocationsEnabled}
        onClick={() => {
          if (!serviceLocationsEnabled) return;
          useAddLocationStore.getState().start({
            locationType: 'service',
            service_location_mode: LocationServiceMode.Prelogon,
          });
          setTimeout(() => {
            navigate({
              to: '/add-location',
            });
          }, 100);
        }}
      />
    </>
  );
};
