import classNames from 'classnames';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { DeviceAvatar } from '../../../../../../shared/defguard-ui/components/Layout/DeviceAvatar/DeviceAvatar';
import { EditButton } from '../../../../../../shared/defguard-ui/components/Layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { EditButtonOptionStyleVariant } from '../../../../../../shared/defguard-ui/components/Layout/EditButton/types';
import { Label } from '../../../../../../shared/defguard-ui/components/Layout/Label/Label';
import { TextContainer } from '../../../../../../shared/defguard-ui/components/Layout/TextContainer/TextContainer';
import { useUserProfileStore } from '../../../../../../shared/hooks/store/useUserProfileStore';
import { ApiToken } from '../../../../../../shared/types';
import { useRenameApiTokenModal } from '../../../../shared/modals/RenameApiTokenModal/useRenameApiTokenModal';
import { useDeleteApiTokenModal } from '../../DeleteApiTokenModal/useDeleteApiTokenModal';

type Props = {
  tokenData: ApiToken;
};
const formatDate = (date: string) => {
  const day = dayjs(date);
  return day.format('DD.MM.YYYY | HH:mm');
};

export const ApiTokenItem = ({ tokenData: token }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.userPage.apiTokens.tokensList;
  const [hovered, setHovered] = useState(false);
  const username = useUserProfileStore((s) => s.userProfile?.user.username);
  const openRenameModal = useRenameApiTokenModal((s) => s.open);
  const openDeleteModal = useDeleteApiTokenModal((s) => s.open);

  const cn = useMemo(
    () =>
      classNames('api-token-item', {
        active: hovered,
      }),
    [hovered],
  );

  return (
    <div
      className={cn}
      onMouseOver={() => setHovered(true)}
      onMouseOut={() => setHovered(false)}
    >
      <header className="item-content">
        <div className="top">
          <DeviceAvatar deviceId={token.id} active={false} />
          <TextContainer text={token.name} />
        </div>
        <div className="bottom">
          <Label>{localLL.common.createdAt()}</Label>
          <TextContainer text={formatDate(token.created_at)} />
        </div>
        <div className="controls">
          <EditButton>
            <EditButtonOption
              text={`${localLL.common.rename()} ${localLL.common.token()}`}
              onClick={() => {
                if (username) {
                  openRenameModal({
                    id: token.id,
                    name: token.name,
                    username,
                  });
                }
              }}
            />
            <EditButtonOption
              styleVariant={EditButtonOptionStyleVariant.WARNING}
              text={`${localLL.common.delete()} ${localLL.common.token()}`}
              onClick={() => {
                if (username) {
                  openDeleteModal({
                    id: token.id,
                    name: token.name,
                    username,
                  });
                }
              }}
            />
          </EditButton>
        </div>
      </header>
    </div>
  );
};
