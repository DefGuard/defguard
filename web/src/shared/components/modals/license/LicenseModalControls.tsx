import { Fragment } from 'react/jsx-runtime';
import { m } from '../../../../paraglide/messages';
import { externalLink } from '../../../constants';
import { Button } from '../../../defguard-ui/components/Button/Button';
import { IconKind } from '../../../defguard-ui/components/Icon';
import { SizedBox } from '../../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../defguard-ui/types';
import { closeModal } from '../../../hooks/modalControls/modalsSubjects';
import type { ModalNameValue } from '../../../hooks/modalControls/modalTypes';
import { Controls } from '../../Controls/Controls';

type Props = {
  modalName: ModalNameValue;
  linkText?: string;
};

export const LicenseModalControls = ({ modalName, linkText }: Props) => {
  return (
    <Fragment>
      <SizedBox height={ThemeSpacing.Xl2} />
      <Controls>
        <div className="right">
          <Button
            variant="secondary"
            text={m.controls_cancel()}
            onClick={() => {
              closeModal(modalName);
            }}
          />
          <a
            target="_blank"
            rel="noopener noreferrer"
            href={externalLink.defguard.pricing}
          >
            <Button
              text={linkText ?? `Check our plans`}
              iconRight={IconKind.OpenInNewWindow}
            />
          </a>
        </div>
      </Controls>
    </Fragment>
  );
};
