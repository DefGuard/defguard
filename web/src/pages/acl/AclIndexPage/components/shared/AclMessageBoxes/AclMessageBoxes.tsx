import './style.scss';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { RenderMarkdown } from '../../../../../../shared/components/Layout/RenderMarkdown/RenderMarkdown';
import { MessageBox } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { AclAliasKind, NetworkAccessType } from '../../../../types';
import { AclAliasKindIcon } from '../AclAliasKindIcon';
import { NetworkAccessTypeIcon } from '../NetworkAccessTypeIcon';

type Props = {
  message: 'acl-alias-kind' | 'acl-network-access';
  dismissable?: boolean;
};

export const AclMessageBoxes = ({ message, dismissable = true }: Props) => {
  const { LL } = useI18nContext();
  const aliasKindLL = LL.acl.messageBoxes.aclAliasKind;
  const networkAccessLL = LL.acl.messageBoxes.networkSelectionIndicatorsHelper;

  switch (message) {
    case 'acl-alias-kind':
      return (
        <MessageBox
          className="acl-explain-message-box"
          type={MessageBoxType.INFO}
          dismissId={dismissable ? 'acl-alias-kind-help' : undefined}
        >
          <ul>
            <li>
              <AclAliasKindIcon kind={AclAliasKind.DESTINATION} />
              <p>{`${aliasKindLL.destination.name()} — ${aliasKindLL.destination.description()}`}</p>
            </li>
            <li>
              <AclAliasKindIcon kind={AclAliasKind.COMPONENT} />
              <p>{`${aliasKindLL.component.name()} — ${aliasKindLL.component.description()}`}</p>
            </li>
          </ul>
        </MessageBox>
      );
    case 'acl-network-access':
      return (
        <MessageBox
          className="acl-explain-message-box"
          type={MessageBoxType.INFO}
          dismissId={dismissable ? 'acl-create-network-selection-help' : undefined}
        >
          <ul>
            <li>
              <NetworkAccessTypeIcon type={NetworkAccessType.DENIED} />
              <p>-&nbsp;</p>
              <RenderMarkdown content={networkAccessLL.denied()} />
            </li>
            <li>
              <NetworkAccessTypeIcon type={NetworkAccessType.ALLOWED} />
              <p>-&nbsp;</p>
              <RenderMarkdown content={networkAccessLL.allowed()} />
            </li>
            <li>
              <NetworkAccessTypeIcon type={NetworkAccessType.UNMANAGED} />
              <p>-&nbsp;</p>
              <RenderMarkdown content={networkAccessLL.unmanaged()} />
            </li>
          </ul>
        </MessageBox>
      );
  }
};
