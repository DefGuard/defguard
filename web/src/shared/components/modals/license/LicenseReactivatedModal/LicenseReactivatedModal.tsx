import './style.scss';
import { useEffect, useState } from 'react';
import { m } from '../../../../../paraglide/messages';
import { AppText } from '../../../../defguard-ui/components/AppText/AppText';
import { Button } from '../../../../defguard-ui/components/Button/Button';
import { Divider } from '../../../../defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../../../defguard-ui/components/SizedBox/SizedBox';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../../../defguard-ui/types';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../hooks/modalControls/modalTypes';
import { Controls } from '../../../Controls/Controls';
import { LicenseModal } from '../../LicenseModal/LicenseModal';
import { LicenseModalSideImage } from '../LicenseModalSideImage/LicenseModalSideImage';

const modalNameKey = ModalName.LicenseReactivated;

export const LicenseReactivatedModal = () => {
  const [isOpen, setOpen] = useState(false);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameKey, () => setOpen(true));
    const closeSub = subscribeCloseModal(modalNameKey, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <LicenseModal
      id="license-reactivated-modal"
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      image={<LicenseModalSideImage variant="reactivated" />}
      lines
    >
      <ModalContent />
    </LicenseModal>
  );
};

const ModalContent = () => {
  return (
    <>
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
        {m.modal_license_reactivated_eyebrow()}
      </AppText>
      <SizedBox height={ThemeSpacing.Xs} />
      <AppText font={TextStyle.TTitleH4} color={ThemeVariable.FgDefault}>
        {m.modal_license_reactivated_title()}
      </AppText>
      <Divider spacing={ThemeSpacing.Xl} />
      <AppText font={TextStyle.TBodyPrimary500} color={ThemeVariable.FgFaded}>
        {m.modal_license_reactivated_lead()}
      </AppText>
      <SizedBox height={ThemeSpacing.Lg} />
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgNeutral}>
        {m.modal_license_reactivated_body()}
      </AppText>
      <SizedBox height={ThemeSpacing.Lg} />
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgNeutral}>
        {m.modal_license_reactivated_note()}
      </AppText>
      <SizedBox height={ThemeSpacing.Xl2} />
      <Controls>
        <div className="right">
          <Button
            text={m.modal_license_reactivated_submit()}
            onClick={() => {
              closeModal(modalNameKey);
            }}
          />
        </div>
      </Controls>
    </>
  );
};
