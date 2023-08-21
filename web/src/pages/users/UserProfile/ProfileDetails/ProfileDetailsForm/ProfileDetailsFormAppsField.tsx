import { useI18nContext } from '../../../../../i18n/i18n-react';
import { Label } from '../../../../../shared/defguard-ui/components/Layout/Label/Label';
import NoData from '../../../../../shared/defguard-ui/components/Layout/NoData/NoData';
import { Tag } from '../../../../../shared/defguard-ui/components/Layout/Tag/Tag';
import { OAuth2AuthorizedApps } from '../../../../../shared/types';

interface Props {
  value: OAuth2AuthorizedApps[];
  onChange: (value: OAuth2AuthorizedApps[]) => void;
}

export const ProfileDetailsFormAppsField = ({ value, onChange }: Props) => {
  const { LL } = useI18nContext();
  if (!value.length) {
    return (
      <>
        <Label>{LL.userPage.userDetails.fields.apps.label()}</Label>
        <NoData customMessage={LL.userPage.userDetails.fields.apps.noData()} />
      </>
    );
  }

  return (
    <>
      <Label>{LL.userPage.userDetails.fields.apps.label()}</Label>
      <div className="tags">
        {value.map((app) => (
          <Tag
            key={app.oauth2client_id}
            text={app.oauth2client_name}
            onDispose={() =>
              onChange(value.filter((a) => a.oauth2client_id !== app.oauth2client_id))
            }
            disposable
          />
        ))}
      </div>
    </>
  );
};
