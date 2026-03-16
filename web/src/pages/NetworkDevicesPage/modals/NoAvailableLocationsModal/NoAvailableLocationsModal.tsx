import { useEffect, useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { m } from '../../../../paraglide/messages';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../../../shared/defguard-ui/types';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../shared/hooks/modalControls/modalTypes';
import { LicenseModal } from '../../../../shared/components/modals/LicenseModal/LicenseModal';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';

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
    <LicenseModal
      id="no-available-locations-modal"
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {}}
    >
      <AppText font={TextStyle.TTitleH4} color={ThemeVariable.FgDefault}>
        {m.modal_no_available_locations_title()}
      </AppText>
      <Divider spacing={ThemeSpacing.Xl} />
      <AppText font={TextStyle.TBodyPrimary500} color={ThemeVariable.FgFaded}>
        {m.modal_no_available_locations_body()}
      </AppText>
      <SizedBox height={ThemeSpacing.Lg} />
      <AppText font={TextStyle.TBodyXs400} color={ThemeVariable.FgMuted}>
        {m.modal_no_available_locations_hint()}
      </AppText>
      <Controls>
        <div className="right">
          <Button
            text={m.controls_close()}
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
    </LicenseModal>
  );
};
