import { m } from '../../../../../../paraglide/messages';
import type { StartEnrollmentResponse } from '../../../../../../shared/api/types';
import { Controls } from '../../../../../../shared/components/Controls/Controls';
import { Button } from '../../../../../../shared/defguard-ui/components/Button/Button';
import { CopyField } from '../../../../../../shared/defguard-ui/components/CopyField/CopyField';
import { SizedBox } from '../../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../../shared/defguard-ui/types';
import { closeModal } from '../../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../../shared/hooks/modalControls/modalTypes';
import './style.scss';

type Props = {
  enrollmentData: StartEnrollmentResponse;
};

export const DeliverTokenStep = ({ enrollmentData }: Props) => {
  return (
    <div id="add-new-device-delivery-step">
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

      <Controls>
        <div className="right">
          <Button
            text={m.controls_close()}
            onClick={() => closeModal(ModalName.AddNewDevice)}
            variant="primary"
          />
        </div>
      </Controls>
    </div>
  );
};
