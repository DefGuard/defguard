import { useEffect, useState } from 'react';
import { AppText } from '../../../../defguard-ui/components/AppText/AppText';
import { Divider } from '../../../../defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../../../defguard-ui/components/SizedBox/SizedBox';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../../../defguard-ui/types';
import { isPresent } from '../../../../defguard-ui/utils/isPresent';
import {
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../hooks/modalControls/modalTypes';
import type { OpenLicenseLimitConflictModal } from '../../../../hooks/modalControls/types';
import { LicenseModal } from '../../LicenseModal/LicenseModal';
import { LicenseModalControls } from '../LicenseModalControls';
import { LicenseModalSideImage } from '../LicenseModalSideImage/LicenseModalSideImage';

const modalNameKey = ModalName.LicenseLimitConflict;

export const LicenseLimitConflictModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<OpenLicenseLimitConflictModal | null>(null);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameKey, (data) => {
      setModalData(data);
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
      id="license-limit-conflict-modal"
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {
        setModalData(null);
      }}
      image={<LicenseModalSideImage variant="expired" />}
      lines
    >
      {isPresent(modalData) && <ModalContent {...modalData} />}
    </LicenseModal>
  );
};

const ModalContent = ({ conflicts }: OpenLicenseLimitConflictModal) => {
  return (
    <>
      <AppText
        font={TextStyle.TBodySm400}
        color={ThemeVariable.FgMuted}
      >{`Plan limits don’t match`}</AppText>
      <SizedBox height={ThemeSpacing.Xs} />
      <AppText
        font={TextStyle.TTitleH4}
        color={ThemeVariable.FgDefault}
      >{`License cannot be applied`}</AppText>
      <Divider spacing={ThemeSpacing.Xl} />
      <AppText
        font={TextStyle.TBodyPrimary500}
        color={ThemeVariable.FgFaded}
      >{`The license you're trying to use allows fewer resources that your current setup is using.`}</AppText>
      <SizedBox height={ThemeSpacing.Lg} />
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgNeutral}>
        {`To apply this license, first reduce your usage so it fits within the license limits.`}
      </AppText>
      <SizedBox height={ThemeSpacing.Lg} />
      {conflicts.length > 0 && (
        <div>
          {conflicts.map((conflict) => (
            <AppText
              key={conflict.label}
              font={TextStyle.TBodySm400}
              color={ThemeVariable.FgNeutral}
            >{`${conflict.label}: ${conflict.current} used, ${conflict.limit} allowed`}</AppText>
          ))}
        </div>
      )}
      <SizedBox height={ThemeSpacing.Lg} />
      <AppText font={TextStyle.TBodyXs400} color={ThemeVariable.FgMuted}>
        {`No changes were made to your current configuration.`}
      </AppText>
      <LicenseModalControls modalName={modalNameKey} />
    </>
  );
};
