import { EditButton } from '../../../../shared/components/layout/EditButton/EditButton';
import { EditButtonOption } from '../../../../shared/components/layout/EditButton/EditButtonOption';
import { RowBox } from '../../../../shared/components/layout/RowBox/RowBox';
import { useUserProfileV2Store } from '../../../../shared/hooks/store/useUserProfileV2Store';

export const UserAuthInfoRecovery = () => {
  const user = useUserProfileV2Store((store) => store.user);
  const isMe = useUserProfileV2Store((store) => store.isMe);
  const editMode = useUserProfileV2Store((store) => store.editMode);
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
              <span>Static</span>
              <EditButton>
                <EditButtonOption text="Generate recovery codes" />
              </EditButton>
            </div>
          </RowBox>
        </>
      ) : (
        <>
          <div className="row">
            <p>Recovery codes</p>
            <p className="info">Static</p>
          </div>
        </>
      )}
    </section>
  );
};
