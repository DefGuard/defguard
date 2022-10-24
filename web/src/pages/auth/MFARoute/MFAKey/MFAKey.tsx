import Button from '../../../../shared/components/layout/Button/Button';

export const MFAKey = () => {
  return (
    <>
      <p>When you are ready to authenticate, press the button below.</p>
      <Button text="Use security key" />
      <div className="mfa-choices">
        <Button text="Use authenticator app instead" />
        <Button text="Use your wallet instead" />
      </div>
    </>
  );
};
