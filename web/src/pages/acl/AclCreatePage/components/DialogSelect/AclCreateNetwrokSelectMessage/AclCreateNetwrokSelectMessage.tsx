import './style.scss';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { RenderMarkdown } from '../../../../../../shared/components/Layout/RenderMarkdown/RenderMarkdown';
import { ActivityIcon } from '../../../../../../shared/defguard-ui/components/icons/ActivityIcon/ActivityIcon';
import { ActivityIconVariant } from '../../../../../../shared/defguard-ui/components/icons/ActivityIcon/types';
import { MessageBox } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';

export const AclCreateNetworkSelectMessage = () => {
  const { LL } = useI18nContext();
  const localLL = LL.acl.createPage.infoBox.networkSelectionIndicatorsHelper;

  return (
    <MessageBox
      className="acl-network-selection-help"
      type={MessageBoxType.INFO}
      dismissId="acl-create-network-selection-help-message"
    >
      <div className="indicators-help">
        <div>
          <div className="icon-wrapper">
            <ActivityIcon status={ActivityIconVariant.ERROR_FILLED} />
          </div>
          <p>{}</p>
          <RenderMarkdown content={`\\- ${localLL.denied()}`} />
        </div>
        <div>
          <div className="icon-wrapper">
            <ActivityIcon status={ActivityIconVariant.CONNECTED} />
          </div>
          <RenderMarkdown content={`\\- ${localLL.allowed()}`} />
        </div>
        <div>
          <div className="icon-wrapper">
            <ActivityIcon status={ActivityIconVariant.BLANK} />
          </div>
          <RenderMarkdown content={`\\- ${localLL.unmanaged()}`} />
        </div>
      </div>
    </MessageBox>
  );
};
