import { useNavigate } from '@tanstack/react-router';
import { useEffect, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Modal } from '../../../../shared/defguard-ui/components/Modal/Modal';
import { TextStyle } from '../../../../shared/defguard-ui/types';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';

const modalNameValue = ModalName.NoAvailableLocations;

export const NoAvailableLocationsModal = () => {
  const [isOpen, setOpen] = useState(false);
  const navigate = useNavigate();

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
    <Modal
      id="no-available-locations-modal"
      size="small"
      title={m.modal_no_available_locations_title()}
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {}}
    >
      <AppText font={TextStyle.TBodySm400}>
        {m.modal_no_available_locations_body()}
      </AppText>
      <Controls>
        <div className="right">
          <Button
            text={m.controls_cancel()}
            variant="secondary"
            onClick={() => setOpen(false)}
          />
          <Button
            text={m.modal_no_available_locations_go_to_locations()}
            onClick={() => {
              setOpen(false);
              navigate({ to: '/locations' });
            }}
          />
        </div>
      </Controls>
    </Modal>
  );
};
