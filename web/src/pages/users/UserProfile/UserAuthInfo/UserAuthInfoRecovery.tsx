import { RowBox } from '../../../../shared/components/layout/RowBox/RowBox';
import { useUserProfileStore } from '../../../../shared/hooks/store/useUserProfileStore';

export const UserAuthInfoRecovery = () => {
  // const user = useUserProfileV2Store((store) => store.user);
  const isMe = useUserProfileStore((store) => store.isMe);
  const editMode = useUserProfileStore((store) => store.editMode);
  const user = useUserProfileStore((store) => store.user);

  if (!user) return null;
  return (
    <section className="recovery">
      <header>
        <h3>Recovery options</h3>
      </header>
      {editMode && isMe ? (
        <>
          <RowBox>
            <p>Recovery Codes</p>
            <div className="right">
              {user.mfa_enabled && (
                <>
                  <span>Viewed</span>
                </>
              )}
            </div>
          </RowBox>
        </>
      ) : (
        <>
          <div className="row">
            <p>Recovery codes</p>
            <p className="info">{user.mfa_enabled && 'Viewed'}</p>
          </div>
        </>
      )}
    </section>
  );
};
