import { useI18nContext } from '../../../../i18n/i18n-react';
import { RowBox } from '../../../../shared/components/layout/RowBox/RowBox';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';

export const UserAuthInfoRecovery = () => {
  const { LL } = useI18nContext();
  const isMe = useUserProfileStore((store) => store.isMe);
  const editMode = useUserProfileStore((store) => store.editMode);
  const user = useUserProfileStore((store) => store.user);

  if (!user) return null;
  return (
    <section className="recovery">
      <header>
        <h3>{LL.userPage.userAuthInfo.recovery.header()}</h3>
      </header>
      {editMode && isMe ? (
        <>
          <RowBox>
            <p>{LL.userPage.userAuthInfo.recovery.codes.label()}</p>
            <div className="right">
              {user.mfa_enabled && (
                <>
                  <span>
                    {LL.userPage.userAuthInfo.recovery.codes.viewed()}
                  </span>
                </>
              )}
            </div>
          </RowBox>
        </>
      ) : (
        <>
          <div className="row">
            <p>{LL.userPage.userAuthInfo.recovery.codes.label()}</p>
            <p className="info">
              {user.mfa_enabled &&
                LL.userPage.userAuthInfo.recovery.codes.viewed()}
            </p>
          </div>
        </>
      )}
    </section>
  );
};
