import { useMemo } from 'react';
import { m } from '../../../../../../paraglide/messages';
import type { StartEnrollmentResponse } from '../../../../../../shared/api/types';
import { externalLink } from '../../../../../../shared/constants';
import { Button } from '../../../../../../shared/defguard-ui/components/Button/Button';
import { CopyField } from '../../../../../../shared/defguard-ui/components/CopyField/CopyField';
import { Divider } from '../../../../../../shared/defguard-ui/components/Divider/Divider';
import { ModalControls } from '../../../../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../../shared/defguard-ui/types';
import { QRCodeCanvas } from 'qrcode.react';
import './style.scss';
import { isPresent } from '../../../../../../shared/defguard-ui/utils/isPresent';

type Props = {
  enrollmentData: StartEnrollmentResponse;
  onClose: () => void;
};

export const DeliveryTokenStep = ({ enrollmentData, onClose }: Props) => {
  const qrData = useMemo(() => {
    if (!enrollmentData) return null;
    return btoa(
      JSON.stringify({
        url: enrollmentData.enrollment_url,
        token: enrollmentData.enrollment_token,
      }),
    );
  }, [enrollmentData.enrollment_url, enrollmentData.enrollment_token]);

  return (
    <div id="add-new-device-delivery-step">
      <div className="download-pdf">
        <p className="section-title">{m.modal_add_new_device_delivery_pdf_title()}</p>
        <p className="section-subtitle">{m.modal_add_new_device_delivery_pdf_subtitle()}</p>
        <SizedBox height={ThemeSpacing.Lg} />
        <Button
          text={m.modal_add_new_device_delivery_download_pdf()}
          variant="outlined"
          iconLeft="download"
          onClick={() => {
            // TODO: PDF download – to be implemented
          }}
        />
      </div>
      <Divider orientation="horizontal" text={m.misc_or()} />
      <div className="share-credentials">
        <p className="section-title">{m.modal_add_new_device_delivery_title()}</p>
        <SizedBox height={ThemeSpacing.Lg} />
        <CopyField
          label={m.modal_add_user_enrollment_form_label_instance_url()}
          copyTooltip={m.controls_copy_clipboard()}
          text={enrollmentData.enrollment_url}
        />
        <SizedBox height={ThemeSpacing.Lg} />
        <CopyField
          label={m.modal_add_user_enrollment_form_label_token()}
          copyTooltip={m.controls_copy_clipboard()}
          text={enrollmentData.enrollment_token}
        />
      </div>
          <SizedBox height={ThemeSpacing.Xs} />

      <Divider orientation="horizontal" />
          <SizedBox height={ThemeSpacing.Xs} />

      <div className="qr-section">
        <div className="qr">
          {isPresent(qrData) && <QRCodeCanvas value={qrData} size={165} />}
        </div>
        <div className="mobile-download">
          <p>{m.modal_add_new_device_delivery_qr_scan()}</p>
          <SizedBox height={ThemeSpacing.Lg} />
          <div className="links">
            <a
              href={externalLink.client.mobile.google}
              target="_blank"
              rel="noopener noreferrer"
            >
              <Button variant="outlined" iconLeft="android" text="Google Play" />
            </a>
            <a
              href={externalLink.client.mobile.apple}
              target="_blank"
              rel="noopener noreferrer"
            >
              <Button variant="outlined" iconLeft="apple" text="Apple Store" />
            </a>
          </div>
        </div>
      </div>
      <ModalControls
        submitProps={{
          text: m.controls_close(),
          onClick: onClose,
        }}
      />
    </div>
  );
};
