import { useEffect, useState } from 'react';
import type { LicenseInfo } from '../../../../api/types';
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
import { LicenseModal } from '../../LicenseModal/LicenseModal';
import { LicenseModalControls } from '../LicenseModalControls';

const modalNameKey = ModalName.LicenseExpired;

export const LicenseExpiredModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [license, setLicense] = useState<LicenseInfo | null>(null);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameKey, (data) => {
      setOpen(true);
      setLicense(data.license);
    });
    const closeSub = subscribeCloseModal(modalNameKey, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);

  return (
    <LicenseModal
      id="license-expired-modal"
      isOpen={isOpen}
      onClose={() => setOpen(false)}
      afterClose={() => {
        setLicense(null);
      }}
    >
      {isPresent(license) && <ModalContent license={license} />}
    </LicenseModal>
  );
};

const ModalContent = ({ license }: { license: LicenseInfo }) => {
  return (
    <>
      <AppText
        font={TextStyle.TBodySm400}
        color={ThemeVariable.FgMuted}
      >{`Action required.`}</AppText>
      <SizedBox height={ThemeSpacing.Xs} />
      <AppText
        font={TextStyle.TTitleH4}
        color={ThemeVariable.FgDefault}
      >{`Your license expired`}</AppText>
      <Divider spacing={ThemeSpacing.Xl} />
      <AppText
        font={TextStyle.TBodySm600}
        color={ThemeVariable.FgFaded}
      >{`Your ${license.tier} Plan license has been disabled. Paid features and extended limits are no longer available.`}</AppText>
      <SizedBox height={ThemeSpacing.Lg} />
      <AppText
        font={TextStyle.TBodySm400}
        color={ThemeVariable.FgNeutral}
      >{`To restore full access, please renew your license.`}</AppText>
      <Divider spacing={ThemeSpacing.Lg} />
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgNeutral}>
        {`If you have any questions, please contact our 
support team at `}
        <AppText
          as="a"
          font={TextStyle.TBodySm400}
          color={ThemeVariable.FgAction}
          style={{ textDecoration: 'none' }}
          href="mailto:support@defguard.net"
        >{`support@defguard.net`}</AppText>
      </AppText>
      <LicenseModalControls modalName={modalNameKey} />
    </>
  );
};
