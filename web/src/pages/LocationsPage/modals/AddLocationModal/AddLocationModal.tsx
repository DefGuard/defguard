import { useNavigate } from '@tanstack/react-router';
import { useEffect, useState } from 'react';
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
      title="Selection location type"
      isOpen={isOpen}
      onClose={() => {
        setOpen(false);
      }}
      afterClose={() => {
        setModalData(null);
      }}
    >
      {isPresent(modalData) && <ModalContent modalData={modalData} />}
    </Modal>
  );
};

const ModalContent = ({ modalData }: { modalData: OpenAddLocationModal }) => {
  const navigate = useNavigate();
  const isEnterprise = modalData.license?.tier === 'Enterprise';

  return (
    <>
      <SectionSelect
        image="location"
        content="Set up your location manually by defining all configuration parameters yourself."
        title="Regular location"
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
        badgeProps={!isEnterprise ? enterpriseBadgeProps : undefined}
        image="service-location"
        content="Service locations are a special kind of locations that allow establishing automatic VPN connections on system boot."
        title="Service location (Windows only)"
        data-testid="add-service-location"
        disabled={!isEnterprise}
        onClick={() => {
          if (!isEnterprise) return;
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
