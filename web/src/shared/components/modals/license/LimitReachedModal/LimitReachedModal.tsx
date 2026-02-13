import { useEffect, useState } from 'react';
import { AppText } from '../../../../defguard-ui/components/AppText/AppText';
import { Divider } from '../../../../defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../../../defguard-ui/components/SizedBox/SizedBox';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../../../defguard-ui/types';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../hooks/modalControls/modalTypes';
import { LicenseModal } from '../../LicenseModal/LicenseModal';
import { LicenseModalControls } from '../LicenseModalControls';

const modalNameKey = ModalName.LimitReached;

export const LimitReachedModal = () => {
  const [isOpen, setOpen] = useState(false);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameKey, () => {
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameKey, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <LicenseModal
      id="limit-reached-modal"
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {}}
    >
      <ModalContent />
    </LicenseModal>
  );
};

const ModalContent = () => {
  return (
    <>
      <AppText
        font={TextStyle.TBodySm400}
        color={ThemeVariable.FgMuted}
      >{`Upgrade your current plan.`}</AppText>
      <SizedBox height={ThemeSpacing.Xs} />
      <AppText
        font={TextStyle.TTitleH4}
        color={ThemeVariable.FgDefault}
      >{`You've reached the plan limit.`}</AppText>
      <Divider spacing={ThemeSpacing.Xl} />
      <AppText
        font={TextStyle.TBodyPrimary500}
        color={ThemeVariable.FgFaded}
      >{`To add more users and increase your limits, please upgrade to the Business plan or higher.`}</AppText>
      <SizedBox height={ThemeSpacing.Lg} />
      <AppText
        font={TextStyle.TBodyXs400}
        color={ThemeVariable.FgMuted}
      >{`To compare all available plans and choose the one that fits your needs, click the button below.`}</AppText>
      <LicenseModalControls modalName={modalNameKey} />
    </>
  );
};
